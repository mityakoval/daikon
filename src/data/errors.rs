use std::{num::ParseIntError, str::Utf8Error};

use thiserror::Error;

#[derive(Error, Debug)]
pub(crate) enum ParsingError {
    #[error("command array ended unexpectedly")]
    IncompleteRequest,
    #[error("syntax error")]
    MalformedRequest,
    #[error("Non-UTF8 character found")]
    BadEncoding(#[from] Utf8Error),
    #[error("invalid command")]
    InvalidCommand(String),
    #[error("value is not an integer or out of range")]
    NotAnInt(#[from] ParseIntError),
}

#[derive(Error, Debug)]
pub(crate) enum DaikonError {
    #[error(transparent)]
    Parsing(#[from] ParsingError),
}

pub(crate) type Result<T> = std::result::Result<T, DaikonError>;
