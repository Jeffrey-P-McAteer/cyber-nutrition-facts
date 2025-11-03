

pub fn analyze_single_file(path: &std::path::Path, args: &crate::args::Args) -> crate::DynResult<()> {
    // First identify if this is source code, or a binary file.
    // Then dispatch to the correct file type function.
    match tika_magic::from_filepath(path) {
        Some(mime) => {
            if is_pe64(mime) || is_pe32(mime) {
                analyze_single_binary(path)
            }
            else if is_text(mime) {
                analyze_single_source(path)
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

fn is_text(mime: &str) -> bool {
    return mime == "application/octet-stream" || mime == "application/text";
}

pub fn analyze_single_binary(path: &std::path::Path, args: &crate::args::Args) -> crate::DynResult<()> {
    
    println!("TODO analyze_single_binary {:?}", path);

    Ok(())
}

pub fn analyze_single_source(path: &std::path::Path, args: &crate::args::Args) -> crate::DynResult<()> {
    
    println!("TODO analyze_single_source {:?}", path);

    Ok(())
}

