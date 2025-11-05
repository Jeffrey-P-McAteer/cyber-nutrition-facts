
#[derive(Debug, clap::Parser)]
#[command(
    name = "cyber-nutrition-facts",
    version = "1.0",
    about = "A source-code, executable-binary, and web-url security information gathering and reporting utility."
)]
pub struct Args {
    /// Data to be analyzed. Supported types of data are: Single Source-Code file, Single PE32, PE32+ .exe binary, single ELF binary, and a web HTTP/S URL.
    pub input: AnalysisInput,

    /// File path where the output .pdf report will be placed. If none is provided, data is written to STDOUT.
    #[arg(short, long)]
    pub output_report: Option<std::path::PathBuf>,

    /// Report Style. Valid ReportStyles are [t|terse, n|normal, d|detailed, o|overflowing]. Pass "--style help" to list all options.
    #[arg(short, long, default_value = "normal")]
    pub style: ReportStyle,
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
            return Ok(AnalysisInput::from(path));
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


#[derive(Debug, Clone)]
pub enum ReportStyle {
    Terse,
    Normal,
    Detailed,
    Overflowing,
}

impl ReportStyle {
    pub fn description(&self) -> String {
        match self {
            ReportStyle::Terse => "Only the top-level summary for each type of information available is printed. Missing types of data are omitted entirely.".into(),
            ReportStyle::Normal => "Everything in Terse are printed, plus MISSING/UNKNOWN for unavailable data. Additionally one level under each top-level summary is printed.".into(),
            ReportStyle::Detailed => "Everything in Normal plus parentheticals, sources of data, and other partially-summarized details which we can not algorithmically decide and are generally weighted low or discarded during summarization.".into(),
            ReportStyle::Overflowing => "Everything we see, before any summarization processing takes place.".into(),
        }
    }
    pub fn named_description(&self) -> String {
        match self {
            ReportStyle::Terse => format!("t|terse: {}", self.description()),
            ReportStyle::Normal => format!("n|normal: {}", self.description()),
            ReportStyle::Detailed => format!("d|detailed: {}", self.description()),
            ReportStyle::Overflowing => format!("o|overflowing: {}", self.description()),
        }
    }
}

impl std::str::FromStr for ReportStyle {
    type Err = String;

    #[allow(irrefutable_let_patterns)]
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_lowercase();
        if s == "t" || s == "terse" {
            Ok(ReportStyle::Terse)
        }
        else if s == "n" || s == "normal" {
            Ok(ReportStyle::Normal)
        }
        else if s == "d" || s == "detailed" {
            Ok(ReportStyle::Detailed)
        }
        else if s == "o" || s == "overflowing" || s == "overflow" {
            Ok(ReportStyle::Overflowing)
        }
        else {
            let mut style_detail_lines = String::with_capacity(2048);

            style_detail_lines.push_str("   - ");
            style_detail_lines.push_str(ReportStyle::Terse.named_description().as_str());
            style_detail_lines.push_str("\n");
            style_detail_lines.push_str("   - ");

            style_detail_lines.push_str(ReportStyle::Normal.named_description().as_str());
            style_detail_lines.push_str("\n");

            style_detail_lines.push_str("   - ");
            style_detail_lines.push_str(ReportStyle::Detailed.named_description().as_str());
            style_detail_lines.push_str("\n");

            style_detail_lines.push_str("   - ");
            style_detail_lines.push_str(ReportStyle::Overflowing.named_description().as_str());
            // style_detail_lines.push_str("\n");

            Err(format!("Unknown ReportStyle: '{s}'.\nValid ReportStyles are [t|terse, n|normal, d|detailed, o|overflowing]:\n{}", style_detail_lines))
        }
    }
}



