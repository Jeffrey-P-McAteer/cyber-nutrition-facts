// src/main.rs
use capstone::arch::x86::X86OperandType;
use capstone::arch::ArchOperand;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::PathBuf;

use capstone::arch::x86::X86Insn;
use capstone::prelude::*;
use object::{Object, ObjectSection, ObjectSymbol, SymbolKind};

pub fn print_tree_of_elf(path: &std::path::Path, root_choice: &str) -> crate::DynResult<()> {
    // let path = PathBuf::from(&args[1]);
    // let root_choice = args.get(2).map(|s| s.as_str()).unwrap_or("main");

    let data = fs::read(&path)?;
    let file = object::File::parse(&*data)?;

    // Find .text section
    let text_section = file
        .sections()
        .find(|s| s.name().unwrap_or("") == ".text")
        .expect("No .text section found");
    let text_addr = text_section.address();
    let text_bytes = text_section.data()?;

    // Collect symbols (text symbols)
    // symbol_map: addr -> (name, size)
    let mut symbol_map: HashMap<u64, (String, u64)> = HashMap::new();
    for symbol in file.symbols() {
        if symbol.kind() == SymbolKind::Text {
            match symbol.name() {
                Ok(name) => {
                    let addr = symbol.address();
                    let size = symbol.size();
                    symbol_map.insert(addr, (name.to_string(), size));
                }
                Err(e) => {
                    eprintln!("{:?}", e);
                }
            }
        }
    }

    // If no symbol table entries for functions, fallback: still we'll create synthetic names for addresses we find.

    // Build address->function ranges using symbol sizes or heuristic (next symbol start or text end)
    let mut starts: Vec<u64> = symbol_map.keys().copied().collect();
    starts.sort_unstable();

    // Build function ranges (start -> end)
    let mut func_ranges: HashMap<u64, u64> = HashMap::new();
    for (i, &start) in starts.iter().enumerate() {
        let end = if let Some((_, size)) = symbol_map.get(&start) {
            if *size > 0 {
                start + *size
            } else {
                // size 0 -> use next symbol or text end
                starts.get(i + 1).copied().unwrap_or(text_addr + text_bytes.len() as u64)
            }
        } else {
            starts.get(i + 1).copied().unwrap_or(text_addr + text_bytes.len() as u64)
        };
        func_ranges.insert(start, end);
    }

    // If symbol_map is empty, create one synthetic entry: whole .text as single function at text_addr
    if symbol_map.is_empty() {
        symbol_map.insert(text_addr, (format!("sub_{:x}", text_addr), text_bytes.len() as u64));
        func_ranges.insert(text_addr, text_addr + text_bytes.len() as u64);
    }

    // Build a map from any address -> function start (so we can classify targets)
    let mut addr_to_func: HashMap<u64, u64> = HashMap::new();
    for (&start, &end) in &func_ranges {
        let mut a = start;
        while a < end {
            addr_to_func.insert(a, start);
            a += 1;
        }
    }

    // Prepare capstone for x86_64
    let cs = Capstone::new()
        .x86()
        .mode(arch::x86::ArchMode::Mode64)
        .detail(true)
        .build()
        .expect("Failed to create Capstone handle");

    // Helper: disassemble a function range and extract direct call targets
    let mut calls_map: HashMap<u64, Vec<u64>> = HashMap::new();

    for (&start, &end) in &func_ranges {
        if end <= start {
            continue;
        }
        // extract bytes for this function: convert virtual addresses to offset within .text
        if start < text_addr {
            continue;
        }
        let offset = (start - text_addr) as usize;
        let size = (end - start) as usize;
        if offset + size > text_bytes.len() {
            continue;
        }
        let bytes = &text_bytes[offset..offset + size];
        // disasm
        let insns = match cs.disasm_all(bytes, start) {
            Ok(i) => i,
            Err(_) => continue,
        };
        let mut callees: Vec<u64> = Vec::new();
        for insn in insns.iter() {
    if insn.id().0 == X86Insn::X86_INS_CALL as u32 {
        if let Ok(detail) = cs.insn_detail(&insn) {
            let arch_detail = detail.arch_detail();
            for op in arch_detail.operands() {
                if let ArchOperand::X86Operand(x86_op) = op {
                    match x86_op.op_type {
                        X86OperandType::Imm(imm) => {
                            // Direct CALL target
                            callees.push(imm as u64);
                        }
                        _ => {
                            // Indirect calls or others we can't statically resolve
                        }
                    }
                }
            }
        }
    }
}
        // dedupe callees
        callees.sort_unstable();
        callees.dedup();
        calls_map.insert(start, callees);
    }

    // Build a symbol-lookup function for nicer printing
    let get_name = |addr: u64| -> String {
        // direct symbol exact match
        if let Some((name, _)) = symbol_map.get(&addr) {
            return name.clone();
        }
        // if addr falls inside a known function, return its symbol
        if let Some(func_start) = addr_to_func.get(&addr) {
            if let Some((name, _)) = symbol_map.get(func_start) {
                return format!("{}+0x{:x}", name, addr - func_start);
            } else {
                return format!("sub_{:x}+0x{:x}", func_start, addr - func_start);
            }
        }
        // else just hex
        format!("sub_{:x}", addr)
    };

    // find root
    let root_addr = find_root(&file, &symbol_map, root_choice, text_addr);
    if root_addr.is_none() {
        eprintln!("Could not find requested root '{}'", root_choice);
        std::process::exit(3);
    }
    let root_addr = root_addr.unwrap();

    // Print tree (DFS)
    let mut visited: HashSet<u64> = HashSet::new();
    print_tree(
        root_addr,
        0,
        &mut visited,
        &calls_map,
        &get_name,
    );

    Ok(())
}

fn find_root(
    file: &object::File<'_>,
    symbol_map: &HashMap<u64, (String, u64)>,
    root_choice: &str,
    text_addr: u64,
) -> Option<u64> {
    // Try to find named symbol in symbol_map
    for (&addr, (name, _size)) in symbol_map {
        if name == root_choice {
            return Some(addr);
        }
    }

    // fallback: use ELF entry point (e.g., __libc_start_main may be called from there)
    if root_choice == "__libc_start_main" {
        // try to find libc start main symbol; if not present, fall back to entry
        for sym in file.symbols() {
            if let Ok(n) = sym.name() {
                if n == "__libc_start_main" {
                    return Some(sym.address());
                }
            }
        }
    }

    // fallback: entry point -> try map entry into .text
    let entry = file.entry();
    if entry >= text_addr {
        return Some(entry);
    }
    // else try find 'main'
    for (&addr, (name, _)) in symbol_map {
        if name == "main" {
            return Some(addr);
        }
    }
    None
}

fn print_tree(
    node: u64,
    depth: usize,
    visited: &mut HashSet<u64>,
    calls_map: &HashMap<u64, Vec<u64>>,
    get_name: &dyn Fn(u64) -> String,
) {
    for _ in 0..depth {
        print!("  ");
    }
    println!("{} ({:#x})", get_name(node), node);
    if visited.contains(&node) {
        for _ in 0..(depth+1) {
            print!("  ");
        }
        println!("(recursive/cycle detected)");
        return;
    }
    visited.insert(node);
    if let Some(callees) = calls_map.get(&node) {
        for &c in callees {
            print_tree(c, depth + 1, visited, calls_map, get_name);
        }
    } else {
        // no recorded callees
    }
    visited.remove(&node);
}
