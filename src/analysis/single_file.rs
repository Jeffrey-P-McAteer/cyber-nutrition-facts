

pub fn analyze_single_file(path: &std::path::Path, args: &crate::args::Args) -> crate::DynResult<()> {
    // First identify if this is source code, or a binary file.
    // Then dispatch to the correct file type function.
    match tika_magic::from_filepath(path) {
        Some(mime) => {
            if is_pe64(mime) || is_pe32(mime) || is_elf(mime) {
                crate::analysis::single_binary::analyze_single_binary(path, args)
            }
            else if is_text(mime) {
                crate::analysis::single_source::analyze_single_source(path, args)
            }
            else {
                //Err(crate::tracked_err!( format!("{:?} has a MIME of {} which is not supported!", path, mime).into() ).into())
                Err(crate::tracked_err!( format!("{:?} has a MIME of {} which is not supported!", path, mime) ).into())
            }
        }
        None => {
            if hyperpolygot_is_text(path) { // Empty files do this
                crate::analysis::single_source::analyze_single_source(path, args)
            }
            else {
                Err(crate::tracked_err!( format!("Cannot determine type of file at {:?}", path) ).into())
            }
        }
    }
}

pub fn is_pe64(mime: &str) -> bool {
    return mime.contains("application/") && mime.contains("pe64");
}

pub fn is_pe32(mime: &str) -> bool {
    return mime == "application/x-msdownload";
}

pub fn is_elf(mime: &str) -> bool {
    return mime == "application/x-sharedlib" || mime == "application/x-executable";
}

pub fn is_text(mime: &str) -> bool {
    return mime == "application/octet-stream" || mime == "application/text" || 
           mime.starts_with("text/") ||
           mime == "application/x-sh";
}

pub fn hyperpolygot_is_text(path: &std::path::Path) -> bool {
    match hyperpolyglot::detect(path) {
        Ok(Some(detection)) => detection.language().to_lowercase() == "text",
        _ => false
    }
}
