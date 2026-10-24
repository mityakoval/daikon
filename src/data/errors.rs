use std::str::Utf8Error;

use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum ParsingError {
    #[error("command array ended unexpectedly")]
    IncompleteRequest,
    #[error("recieved a malformed command")]
    MalformedRequest,
    #[error("Non-UTF8 character found")]
    BadEncoding(#[from] Utf8Error),
}

#[derive(Error, Debug)]
pub(crate) enum DaikonError {
    #[error(transparent)]
    Parsing(#[from] ParsingError),
}

pub(crate) type Result<T> = std::result::Result<T, DaikonError>;
