

pub fn analyze_single_binary(path: &std::path::Path, args: &crate::args::Args) -> crate::DynResult<()> {
    
    let binary_content_bytes = std::fs::read(path)?;

    let obj = goblin::Object::parse(&binary_content_bytes)?;

    print_referenced_libraries("", path, &obj, args);

    Ok(())
}

pub fn print_referenced_libraries(prefix: &str, path: &std::path::Path, gobj: &goblin::Object, args: &crate::args::Args) {
    match gobj {
        goblin::Object::Elf(elf) => {
            println!("{}= = = = Shared Libraries = = = =", prefix);

            let dynamic_libs = elf.dynamic.as_ref().map(|v| v.get_libraries(&elf.dynstrtab)).unwrap_or_else(|| vec![]);

            let (lib_funcs, symbols_not_found) = match super::elf_lib_lookup::simulate_dynamic_linking(path, None, args) {
                Ok(lf) => lf,
                Err(e) => {
                    eprintln!("{:?}", e);
                    (std::collections::HashMap::new(), Vec::new())
                }
            };


            if dynamic_libs.len() < 1 {
                println!("{}NO LIBRARIES REFERENCED IN elf.dynstrtab", prefix);
            }
            else {
                for lib in dynamic_libs.iter() {
                    let lib = format!("{}", lib);
                    println!("{} - {}", prefix, lib);
                    if let Some(funcs) = lib_funcs.get(&lib) {
                        for func in funcs.iter() {
                            println!("{}   - {}", prefix, func);
                        }
                    }
                }
            }

            if args.style >= crate::args::ReportStyle::Detailed {
                println!("{} {} symbols/functions were not found in ANY shared libraries:", prefix, symbols_not_found.len());
                for not_found_name in symbols_not_found.iter() {
                    println!("{}   - {}", prefix, not_found_name);
                }
            }

        }
        goblin::Object::PE(pe) => {
            println!("{}= = = = Shared Libraries = = = =", prefix);

            let import_libs: Vec<String> = pe.import_data.as_ref().map(|v| (&v.import_data).into_iter().map(|id| id.name.to_string()).collect() ).unwrap_or_else(|| vec![]);
            let mut lib_funcs: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();

            if let Some(import_datas) = &pe.import_data {
                for import_data in import_datas.import_data.iter() {
                    let mut funcs = Vec::new();
                    if let Some(ilt_vec) = &import_data.import_lookup_table {
                        for ilt in ilt_vec.iter() {
                            funcs.push(silte_to_string(ilt));
                        }
                    }
                    lib_funcs.insert(import_data.name.to_string(), funcs);
                }
            }

            if import_libs.len() < 1 {
                println!("{}NO LIBRARIES REFERENCED IN pe.import_data", prefix);
            }
            else {
                for lib in import_libs.iter() {
                    println!("{} - {}", prefix, lib);
                    if let Some(funcs) = lib_funcs.get(lib) {
                        for func in funcs.iter() {
                            println!("{}   - {}", prefix, func);
                        }
                    }
                }
            }
        }
        _ => {
            println!("{} TODO Implement support in print_referenced_libraries for gobj={:?}", prefix, gobj);
        }
    }
}

fn silte_to_string(silte: &goblin::pe::import::SyntheticImportLookupTableEntry) -> String {
    match silte {
        goblin::pe::import::SyntheticImportLookupTableEntry::OrdinalNumber(num) => {
            format!("{} (Only an OrdinalNumber)", num)
        }
        goblin::pe::import::SyntheticImportLookupTableEntry::HintNameTableRVA((num, table_entry)) => {
            format!("{} ({})", table_entry.name, num)
        }
    }
}


