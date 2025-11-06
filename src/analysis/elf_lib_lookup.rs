use goblin::elf::{Elf, sym};
use goblin::elf::sym::STT_NOTYPE;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

/// Default library search paths (you can extend).
const DEFAULT_SEARCH_PATHS: &[&str] = &[
    "/lib",
    "/lib64",
    "/usr/lib",
    "/usr/lib64",
];

/// Parse ELF bytes into goblin::elf::Elf
fn parse_elf_bytes(buf: &[u8]) -> crate::DynResult<Elf> {
    Ok(Elf::parse(buf)?)
}

/// Find a library file named `soname` inside the provided search paths.
/// Returns the first matching path if found.
fn find_library_on_paths(soname: &str, search_paths: &[PathBuf]) -> Option<PathBuf> {
    for dir in search_paths {
        let candidate = dir.join(soname);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// Parse a shared object and return a set of exported symbol names (dynamic symbols
/// that are defined in the DSO, i.e., st_shndx != SHN_UNDEF).
fn exported_symbols_from_so(path: &Path) -> crate::DynResult<HashSet<String>> {
    let buf = fs::read(path)?;
    let elf = parse_elf_bytes(&buf)?;

    let mut exports = HashSet::new();
    // dynsyms generally indicate exported/needed runtime symbols
    for sym_entry in &elf.dynsyms {
        // if st_shndx != 0 (SHN_UNDEF), it's defined in this object
        if sym_entry.st_shndx != goblin::elf::section_header::SHN_UNDEF as usize {
            if let Some(name) = elf.dynstrtab.get_at(sym_entry.st_name) {
                // Skip empty or internal names
                if !name.is_empty() {
                    exports.insert(name.to_string());
                }
            }
        }
    }
    Ok(exports)
}

/// Build a recursive load order from an initial list of sonames (DT_NEEDED),
/// resolving each soname file on disk via `search_paths`. This returns a vector
/// of resolved file paths in the order they should be searched (first -> last).
///
/// We perform a breadth-first recursive discovery while respecting the
/// initial ordering (left-to-right).
fn build_load_order(initial_sonames: &[&str], search_paths: &[PathBuf]) -> crate::DynResult<Vec<PathBuf>> {
    let mut order = Vec::new();
    let mut seen_sonames = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();

    // seed with initial sonames preserving order
    for s in initial_sonames {
        queue.push_back(s.to_string());
    }

    while let Some(soname) = queue.pop_front() {
        if seen_sonames.contains(&soname) {
            continue;
        }
        seen_sonames.insert(soname.clone());

        if let Some(path) = find_library_on_paths(&soname, search_paths) {
            order.push(path.clone());

            // parse the library to find its DT_NEEDED children and queue them
            let buf = fs::read(&path)?;
            let elf = parse_elf_bytes(&buf)?;
            for dep in &elf.libraries {
                if !seen_sonames.contains(*dep) {
                    queue.push_back(dep.to_string());
                }
            }
        } else {
            // library file not found; we still mark it seen to avoid infinite loop,
            // but we warn (returning an error could be another choice)
            eprintln!("warning: could not locate '{}' in provided search paths", soname);
        }
    }

    Ok(order)
}

/// Simulate symbol resolution for `target_path` using the `search_paths`.
///
/// For each undefined dynamic symbol in the target, find the first library in
/// the load order which exports it, and print the mapping.
pub fn simulate_dynamic_linking(target_path: &Path, search_paths: Option<&[PathBuf]>, args: &crate::args::Args) -> crate::DynResult< (std::collections::HashMap<String, Vec<String>>, Vec<String>) > {
    let mut lib_funcs: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    let mut symbols_not_found: Vec<String> = Vec::new();

    let search_paths_vec: Vec<PathBuf> = match search_paths {
        Some(sp) => sp.to_vec(),
        None => DEFAULT_SEARCH_PATHS.iter().map(|s| PathBuf::from(s)).collect(),
    };

    let buf = fs::read(target_path)?;
    let elf = parse_elf_bytes(&buf)?;

    // println!("Target: {}", target_path.display());
    // println!("DT_NEEDED (declared shared libs):");
    // for (i, lib) in elf.libraries.iter().enumerate() {
    //     println!("  [{}] {}", i, lib);
    // }

    // Build full load order (search order) by resolving sonames on disk
    let load_order_files = build_load_order(&elf.libraries, &search_paths_vec)?;

    // if load_order_files.is_empty() {
    //     println!("No resolved DT_NEEDED libraries found in search paths.");
    // } else {
    //     println!("\nResolved load order (first -> last):");
    //     for (i, path) in load_order_files.iter().enumerate() {
    //         println!("  [{}] {}", i, path.display());
    //     }
    // }

    // Parse each library once and cache its exported symbols
    let mut lib_exports: Vec<(PathBuf, HashSet<String>)> = Vec::new();
    for libpath in &load_order_files {
        match exported_symbols_from_so(libpath) {
            Ok(set) => lib_exports.push((libpath.clone(), set)),
            Err(e) => {
                eprintln!(
                    "warning: failed to parse exports from {}: {}",
                    libpath.display(),
                    e
                );
            }
        }
    }

    // Collect undefined dynamic symbols from target (.dynsym where st_shndx == SHN_UNDEF)
    let mut undefined_funcs: Vec<String> = Vec::new();
    for sym_entry in &elf.dynsyms {
        if sym_entry.st_shndx == goblin::elf::section_header::SHN_UNDEF as usize {
            if let Some(name) = elf.dynstrtab.get_at(sym_entry.st_name) {
                if !name.is_empty() {
                    // optional: check symbol type to prefer functions only
                    let typ = sym::st_type(sym_entry.st_info);
                    // We'll include all undefined symbols; optionally filter for STT_FUNC
                    undefined_funcs.push(name.to_string());
                }
            }
        }
    }

    // println!("\nUndefined dynamic symbols (candidates to resolve):");
    // for name in &undefined_funcs {
    //     println!("  {}", name);
    // }

    //println!("\nResolution results:");
    for sym_name in &undefined_funcs {
        let mut found: Option<&PathBuf> = None;
        for (libpath, exports) in &lib_exports {
            if exports.contains(sym_name) {
                found = Some(libpath);
                break;
            }
        }
        match found {
            Some(libpath) => {
                //println!("  {:<40} -> {}", sym_name, libpath.display());
                let lib_path_str = format!("{}", libpath.file_name().unwrap_or(libpath.as_os_str()).to_string_lossy() );
                lib_funcs.entry(lib_path_str)
                         .or_insert_with(Vec::new)
                         .push(sym_name.to_string());
            },
            None => {
                //println!("  {:<40} -> <NOT FOUND in load order>", sym_name);
                symbols_not_found.push(sym_name.to_string());
            }
        }
    }

    Ok( (lib_funcs, symbols_not_found) )
}
