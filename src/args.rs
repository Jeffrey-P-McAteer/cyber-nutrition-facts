
#[derive(Debug, clap::Parser)]
pub struct Args {
    pub input: AnalysisInput,

    #[arg(short, long)]
    pub output_report: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone)]
pub enum AnalysisInput {
    Url(uris::Uri),
    File(std::path::PathBuf),
    Folder(std::path::PathBuf),
    Command(ArgCommand),
}

// This is used to allow a sub-command like capability
#[derive(Debug, Clone)]
pub enum ArgCommand {
    CheckSetup
}

impl std::str::FromStr for AnalysisInput {
    type Err = String;

    #[allow(irrefutable_let_patterns)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Shotgun approach - parse everything, place preference logic below.
        // This way if we have a "may be URI or may be file" we can place business logic there.
        let r_uri = uris::Uri::parse(s);
        let r_path = std::path::PathBuf::from_str(s);
        let r_command = ArgCommand::from_str(s);

        // Ahead-of-line parsing;

        // Commands: These are run first, and unfortunately deny file-paths which match. Sucks, we don't care.
        //           Don't write .pdf reports to the path ./check.
        if let Ok(ref command) = r_command {
            return Ok(AnalysisInput::Command(command.clone()));
        }

        // Paths: if the path parses and points to an existing item, prefer that as it's likely what the user intended.
        if let Ok(ref path) = r_path {
            if path.exists() {
                return Ok(AnalysisInput::from(path.as_path()));
            }
        }
        
        // Regular parse hierarchy; most-detailed formats to least-detailed
        if let Ok(command) = r_command {
            return Ok(AnalysisInput::Command(command));
        }
        else if let Ok(uri) = r_uri {
            Ok(AnalysisInput::Url(uri))
        }
        else if let Ok(path) = r_path {
            if path.exists() {
                return Ok(AnalysisInput::from(path));
            }
            else {
                Err(format!("The path '{:?}' does not exist!", path))
            }
        }
        else {
            let mut all_err_msgs = String::with_capacity(1024);
            if let Err(e) = r_uri {
                all_err_msgs.push_str(format!("While parsing as URL: {:?}\n", e).as_str());
            }
            if let Err(e) = r_path {
                all_err_msgs.push_str(format!("While parsing as Path: {:?}\n", e).as_str());
            }
            Err(all_err_msgs)
        }
    }
}

impl std::str::FromStr for ArgCommand {
    type Err = String;

    #[allow(irrefutable_let_patterns)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        if s == "check" || s == "check-setup" || s == "check_setup" {
            Ok(ArgCommand::CheckSetup)
        }
        else {
            Err(format!("Unknown command: '{s}'. Valid commands are [check, ]"))
        }
    }
}

impl std::convert::From<std::path::PathBuf> for AnalysisInput {
    fn from(value: std::path::PathBuf) -> Self {
        if value.exists() {
            if value.is_file() {
                AnalysisInput::File(value)
            }
            else {
                AnalysisInput::Folder(value)
            }
        }
        else {
            // Assume non-existent file
            AnalysisInput::File(value)
        }
    }
}

impl std::convert::From<&std::path::Path> for AnalysisInput {
    fn from(value: &std::path::Path) -> Self {
        if value.exists() {
            if value.is_file() {
                AnalysisInput::File(value.to_path_buf())
            }
            else {
                AnalysisInput::Folder(value.to_path_buf())
            }
        }
        else {
            // Assume non-existent file
            AnalysisInput::File(value.to_path_buf())
        }
    }
}



