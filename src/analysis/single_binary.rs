

pub fn analyze_single_file(path: &std::path::Path, args: &crate::args::Args) -> crate::DynResult<()> {
    // First identify if this is source code, or a binary file.
    // Then dispatch to the correct file type function.
    match tika_magic::from_filepath(path) {
        Some(mime) => {
            if is_pe64(mime) {
                println!("TODO launch vm and run analysis on a PE64 binary");
                
                Ok(())
            }
            else if is_pe32(mime) {
                println!("TODO launch vm and run analysis on a PE32 binary");
                
                Ok(())
            }
            else {
                Err(format!("{:?} has a MIME of {} which is not supported!", path, mime).into())
            }
        }
        None => {
            Err(format!("Cannot determine type of file at {:?}", path).into())
        }
    }
}

fn is_pe64(mime: &str) -> bool {
    return mime.contains("application/") && mime.contains("pe64");
}

fn is_pe32(mime: &str) -> bool {
    return mime == "application/x-msdownload";
}

pub fn analyze_single_binary(path: &std::path::Path, args: &crate::args::Args) -> crate::DynResult<()> {
    
    println!("TODO analyze_single_binary {:?}", path);

    Ok(())
}

pub fn analyze_single_source(path: &std::path::Path, args: &crate::args::Args) -> crate::DynResult<()> {
    
    println!("TODO analyze_single_source {:?}", path);

    Ok(())
}

