

pub fn analyze_single_file(path: &std::path::Path, args: &crate::args::Args) -> crate::DynResult<()> {
    // First identify if this is source code, or a binary file.
    // Then dispatch to the correct file type function.
    match tika_magic::from_filepath(path) {
        Some(mime) => {
            if is_pe64(mime) || is_pe32(mime) {
                analyze_single_binary(path, args)
            }
            else if is_text(mime) {
                analyze_single_source(path, args)
            }
            else {
                //Err(crate::tracked_err!( format!("{:?} has a MIME of {} which is not supported!", path, mime).into() ).into())
                Err(crate::tracked_err!( format!("{:?} has a MIME of {} which is not supported!", path, mime) ).into())
            }
        }
        None => {
            if hyperpolygot_is_text(path) { // Empty files do this
                analyze_single_source(path, args)
            }
            else {
                Err(crate::tracked_err!( format!("Cannot determine type of file at {:?}", path) ).into())
            }
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

fn hyperpolygot_is_text(path: &std::path::Path) -> bool {
    match hyperpolyglot::detect(path) {
        Ok(Some(detection)) => detection.language().to_lowercase() == "text",
        _ => false
    }
}

pub fn analyze_single_binary(path: &std::path::Path, args: &crate::args::Args) -> crate::DynResult<()> {
    
    println!("TODO analyze_single_binary {:?}", path);

    Ok(())
}

pub fn analyze_single_source(path: &std::path::Path, args: &crate::args::Args) -> crate::DynResult<()> {
    let language_detection = hyperpolyglot::detect(path).map_err(|e| crate::tracked_err!(e))?;
    let lang = language_detection.map(|v| v.language()).unwrap_or_else(|| "Unknown").to_lowercase();

    println!("language_detection = {:?} lang = {:?}", language_detection, lang);
    println!("TODO analyze_single_source {:?}", path);


    print_all_metrics_for_file(path)?;


    Ok(())
}

use std::error::Error;
use std::path::Path;

use rust_code_analysis::{action, get_language_for_file, read_file_with_eol, LANG, Metrics, MetricsCfg};

/// Read `path`, detect language and print/dump all metrics to stdout using rust-code-analysis.
///
/// Returns `Ok(())` on success, or an error boxed as `Box<dyn Error>`.
pub fn print_all_metrics_for_file(path: &Path) -> Result<(), Box<dyn Error>> {
    // 1) detect language from file extension
    let lang: LANG = get_language_for_file(path)
        .ok_or_else(|| format!("could not detect language for file {:?}", path))?;

    // 2) read file bytes (the crate helper ensures EOL)
    let source_vec = read_file_with_eol(path)?
        .ok_or_else(|| format!("file {:?} is empty or could not be read", path))?;

    // 3) prepare MetricsCfg (the action API expects a cfg that contains the path)
    let cfg = MetricsCfg {
        path: path.to_path_buf(),
    };

    // 4) call action::<Metrics> which runs the metrics callback and (per crate examples) dumps metrics
    //    The return type of action::<Metrics> is `Result<(), rust_code_analysis::Error>`
    action::<Metrics>(&lang, source_vec, &cfg.path.clone(), None, cfg)
        .map_err(|e| Box::<dyn Error>::from(format!("metrics action failed: {}", e)))?;

    Ok(())
}
