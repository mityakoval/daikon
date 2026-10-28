use std::{num::ParseIntError, str::Utf8Error};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParsingError {
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
    #[error("unknown command")]
    UnknownCommand,
}

#[derive(Error, Debug)]
pub(crate) enum DaikonError {
    #[error(transparent)]
    Parsing(#[from] ParsingError),
}
