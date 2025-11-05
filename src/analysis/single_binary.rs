
pub fn analyze_single_binary(path: &std::path::Path, args: &crate::args::Args) -> crate::DynResult<()> {
    
    let binary_content_bytes = std::fs::read(path)?;

    let obj = goblin::Object::parse(&binary_content_bytes)?;

    if args.style >= crate::args::ReportStyle::Detailed {
        println!("TODO analyze_single_binary {:?}", path);
        println!("obj = {:?}", obj);
    }
    else {
        println!("TODO pass '--style detailed' or greater for in-development outputs.")
    }

    Ok(())
}



