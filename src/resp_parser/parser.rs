use std::fmt::{Display, Formatter};
use anyhow::{Error};
use thiserror::Error;
use crate::Command;

pub fn parse(input: &str) -> anyhow::Result<Command> {
    if let Some(rest) = input.strip_prefix("*") {

    }
    Err(Error::msg("Unexpected input"))
}

#[derive(Error, Debug)]
enum ParseError {
    UnexpectedInput()
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
