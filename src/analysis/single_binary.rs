
pub fn analyze_single_binary(path: &std::path::Path, args: &crate::args::Args) -> crate::DynResult<()> {
    
    let binary_content_bytes = std::fs::read(path)?;

    let obj = goblin::Object::parse(&binary_content_bytes)?;

    if args.style >= crate::args::ReportStyle::Detailed {
        println!("TODO analyze_single_binary {:?}", path);
        println!("obj = {:?}", obj);
        print_referenced_libraries("", &obj, args);
    }
    else {
        println!("TODO pass '--style detailed' or greater for in-development outputs.");
        print_referenced_libraries("", &obj, args);
    }

    Ok(())
}

pub fn print_referenced_libraries(prefix: &str, gobj: &goblin::Object, args: &crate::args::Args) {
    match gobj {
        goblin::Object::Elf(elf) => {
            println!("{}= = = = Shared Libraries = = = =", prefix);
            let dynamic_libs = elf.dynamic.as_ref().map(|v| v.get_libraries(&elf.dynstrtab)).unwrap_or_else(|| vec![]);
            if dynamic_libs.len() < 1 {
                println!("{}NO LIBRARIES REFERENCED IN dynstrtab", prefix);
            }
            else {
                for lib in dynamic_libs.iter() {
                    println!("{} - {}", prefix, lib)
                }
            }

        }
        goblin::Object::PE(pe) => {

        }
        _ => {
            println!("{} TODO Implement support in print_referenced_libraries for gobj={:?}", prefix, gobj);
        }
    }
}

