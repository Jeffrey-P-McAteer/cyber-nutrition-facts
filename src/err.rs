use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct WithLocationErr {
    pub file: &'static str,
    pub line: u32,
    pub source: Box<dyn Error + Send + Sync>,
}

impl fmt::Display for WithLocationErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}:{}",
            self.source,
            self.file,
            self.line
        )
    }
}

impl Error for WithLocationErr {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&*self.source)
    }
}

#[macro_export]
macro_rules! tracked_err {
    ($err:expr) => {
        crate::err::WithLocationErr {
            file: file!(),
            line: line!(),
            source: ($err).into(),
        }
    };
}
pub use tracked_err;

