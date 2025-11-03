// We have several compiler warnings we don't care about during early development
#![allow(unused_variables, unused_imports)]

use printpdf::*;
use std::fs::File;
use std::io::BufWriter;

type DynResult<T> = Result<T, Box<dyn std::error::Error>>;

mod args;
mod checks;
mod analysis;
mod err; // exports crate::tracked_err!()

fn main() {
    if let Err(e) = fallible_main() {
        println!("{}", e);
        std::process::exit(1);
    }
}

fn fallible_main() -> DynResult<()> {
    use clap::Parser;
    let args = args::Args::parse();

    match args.input {
        args::AnalysisInput::File(ref file_to_analyze) => {
            analysis::analyze_single_file(file_to_analyze, &args)?;
        }
        args::AnalysisInput::Command(args::ArgCommand::CheckSetup) => {
            checks::check_setup(&args)?;
        }
        _ => {
            println!("Unhandled main path, args = {:?}", args);
        }
    }

    Ok(())
}


pub fn create_pdf_from_items(filename: &str, items: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    // Create a PDF document with one page of standard A4 size
    let (doc, page1, layer1) = PdfDocument::new("List PDF", Mm(210.0), Mm(297.0), "Layer 1");
    let current_layer = doc.get_page(page1).get_layer(layer1);

    // Load a built-in font (Helvetica)
    let font = doc.add_builtin_font(BuiltinFont::Helvetica)?;

    // Initial text position (from bottom-left of page)
    let mut y_pos = 280.0; // mm

    for item in items {
        current_layer.use_text(
            *item,
            14.0,                 // font size
            Mm(10.0),             // x position
            Mm(y_pos),            // y position
            &font,
        );
        y_pos -= 10.0; // Move down for next line
        if y_pos < 10.0 { break; } // Avoid writing off page
    }

    // Save to file
    doc.save(&mut BufWriter::new(File::create(filename)?))?;

    Ok(())
}

