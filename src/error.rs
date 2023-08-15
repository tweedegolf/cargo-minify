use thiserror::Error;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("{0}")]
    CommandLine(#[from] gumdrop::Error),

    #[error("invalid command line arguments: {0}")]
    Args(&'static str),
}
