use crate::data::commands::CommandName::{self};
use crate::data::commands::Kommand;
use crate::data::errors::ParsingError::{self, InvalidCommand, MalformedRequest};
use crate::data::resp_types::RespValue;
use crate::parser::input::parse_data_bytes;
use bytes::BytesMut;
use std::collections::VecDeque;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn parse_command_array(input: &BytesMut) -> Result<Kommand, ParsingError> {
    match parse_data_bytes(input.iter().as_slice())?.unwrap() {
        (RespValue::Array(mut command_array), _) => {
            trace!("Parsed array: {:?}", command_array);

            if let Some(RespValue::BulkString(command_bulk_str)) = command_array.pop_front() {
                let command_name = CommandName::from_str(command_bulk_str.to_uppercase().as_str())?;
                match command_name {
                    CommandName::Ping => {
                        require_empty(command_array, command_name)?;
                        Ok(Kommand::Ping)
                    }
                    CommandName::Echo => {
                        let value = command_array.pop_back().unwrap();
                        require_empty(command_array, command_name)?;
                        Ok(Kommand::Echo(value))
                    }
                    CommandName::Set => {
                        let ttl = parse_ttl(&mut command_array)?;
                        let key = parse_key(command_array.pop_front().unwrap())?;
                        let value = command_array.pop_back().unwrap();
                        require_empty(command_array, command_name)?;
                        Ok(Kommand::Set { key, value, ttl })
                    }
                    CommandName::Get => {
                        let key = parse_key(command_array.pop_front().unwrap())?;
                        require_empty(command_array, command_name)?;
                        Ok(Kommand::Get(key))
                    }
                    CommandName::Del => {
                        let keys = parse_keys(&mut command_array)?;
                        require_empty(command_array, command_name)?;
                        Ok(Kommand::Del(keys))
                    }
                    CommandName::Rpush => {
                        let key = parse_key(command_array.pop_front().unwrap())?;
                        let values = command_array;
                        Ok(Kommand::Rpush { key, values })
                    }
                }
            } else {
                Err(MalformedRequest)
            }
        }
        _ => Err(MalformedRequest),
    }
}

fn parse_key(key: RespValue) -> Result<Arc<str>, ParsingError> {
    if let RespValue::BulkString(key) = key {
        Ok(Arc::<str>::from(key.to_owned()))
    } else {
        Err(MalformedRequest)
    }
}

fn parse_keys(command_array: &mut VecDeque<RespValue>) -> Result<Vec<Arc<str>>, ParsingError> {
    let mut keys = Vec::with_capacity(command_array.len());
    while let Some(v) = command_array.pop_front() {
        let RespValue::BulkString(key) = v else {
            return Err(MalformedRequest);
        };
        keys.push(Arc::<str>::from(key.to_owned()));
    }
    Ok(keys)
}

fn parse_ttl(array: &mut VecDeque<RespValue>) -> Result<Option<Duration>, ParsingError> {
    let rem_len = array.len();
    if rem_len < 2 {
        return Ok(None);
    }
    let ttl;
    let units;
    match array[rem_len - 2] {
        RespValue::BulkString(ref _units) => {
            if ["EX", "PX"].contains(&_units.to_uppercase().as_str()) {
                ttl = array.pop_back().unwrap();
                units = array.pop_back().unwrap();
                match units {
                    RespValue::BulkString(units) => {
                        let RespValue::BulkString(ttl) = ttl else {
                            return Err(InvalidCommand(
                                "value is not an integer or out of range".to_string(),
                            ));
                        };

                        let ttl = ttl.parse::<u64>()?;

                        match units.to_uppercase().as_str() {
                            "EX" => Ok(Some(Duration::from_secs(ttl))),
                            "PX" => Ok(Some(Duration::from_millis(ttl))),
                            _ => Err(MalformedRequest),
                        }
                    }
                    _ => Err(MalformedRequest),
                }
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

fn require_empty(
    array: VecDeque<RespValue>,
    command_name: CommandName,
) -> Result<(), ParsingError> {
    if !array.is_empty() {
        return Err(ParsingError::InvalidCommand(format!(
            "wrong number of arguments for '{}' command",
            command_name
        )));
    }
    Ok(())
}
