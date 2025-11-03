
// This list tracks the external tools that cyber-nutrition-facts relies on.
const REQUIRED_BINS_DESCRIPTIONS: &[(&'static str, &'static str)] = &[
    ("qemu-system-x86_64", "This is used to run Docker within a Windows Userspace app"),
];

pub fn check_setup(args: &crate::args::Args) -> crate::DynResult<()> {
    
    let mut missing_bins = Vec::new();
    let mut all_good = true;
    for (bin, reason) in REQUIRED_BINS_DESCRIPTIONS.iter() {
        all_good &= find_and_report_bin_exists(bin, reason, &mut missing_bins);
    }

    if all_good {
        Ok(())
    }
    else {
        Err(format!("The following binaries are missing, please install them and their containing folder to your PATH variable. missing_bins = {:?}", missing_bins).into())
    }
}

pub fn which_with_extensions(binary_name: &str) -> which::Result<std::path::PathBuf> {
    const EXTS: &[&'static str] = &["", ".exe", ".com"];
    for ext in EXTS.iter() {
        if let Ok(binary_path) = which::which(format!("{}{}", binary_name, ext)) {
            return Ok(binary_path);
        }
    }
    // If we are here all we have are failures, so return the first one to keep the which::which error messaging
    which::which(binary_name)
}

// Returns true if binary exists, false if binary does not exist
fn find_and_report_bin_exists(binary_name: &str, reason_txt: &str, missing_bins: &mut Vec<String>) -> bool {
    match which_with_extensions(binary_name) {
        Ok(binary_path) => {
            println!("Found required program '{}' at {:?} - {}", binary_name, binary_path, reason_txt);
            true
        }
        Err(e) => {
            missing_bins.push(binary_name.to_string());
            println!("Cannot find required program {:<18} ({:?}) ({})", format!("'{}'", binary_name), e, reason_txt);
            false
        }
    }
}


