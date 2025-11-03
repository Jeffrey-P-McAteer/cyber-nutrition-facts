
#[derive(Debug, clap::Parser)]
pub struct Args {
    pub input: AnalysisInput,
}

#[derive(Debug, Clone)]
pub enum AnalysisInput {
    Url(uris::Uri),
    File(std::path::PathBuf)
}

impl std::str::FromStr for AnalysisInput {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Err("TODO implement me".into())
    }
}



