use crate::data::commands::Command;
use crate::data::commands::CommandName::{self};
use crate::data::errors::ParsingError::{self, InvalidCommand, MalformedRequest};
use crate::data::errors::ParsingResult;
use crate::data::resp_types::RespValue;
use crate::parser::input::parse_data_bytes;
use bytes::{Bytes, BytesMut};
use std::collections::VecDeque;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn parse_command_array(input: &BytesMut) -> ParsingResult<Command> {
    match parse_data_bytes(input.iter().as_slice())? {
        Some((RespValue::Array(mut command_array), _)) => {
            trace!("Parsed array: {:?}", command_array);

            if let Some(RespValue::BulkString(command_bulk_str)) = command_array.pop_front() {
                let command_name = CommandName::from_str(command_bulk_str.to_uppercase().as_str())?;
                match command_name {
                    CommandName::Ping => {
                        require_empty(command_array, command_name)?;
                        Ok(Command::Ping)
                    }
                    CommandName::Echo => {
                        let value = parse_value(command_array.pop_back())?;
                        require_empty(command_array, command_name)?;
                        Ok(Command::Echo(value))
                    }
                    CommandName::Set => {
                        let ttl = parse_ttl(&mut command_array)?;
                        let key = parse_key(command_array.pop_front())?;
                        let value = parse_value(command_array.pop_back())?;
                        require_empty(command_array, command_name)?;
                        Ok(Command::Set { key, value, ttl })
                    }
                    CommandName::Get => {
                        let key = parse_key(command_array.pop_front())?;
                        require_empty(command_array, command_name)?;
                        Ok(Command::Get(key))
                    }
                    CommandName::Del => {
                        let keys = parse_keys(&mut command_array)?;
                        require_empty(command_array, command_name)?;
                        Ok(Command::Del(keys))
                    }
                    CommandName::Rpush => {
                        let key = parse_key(command_array.pop_front())?;
                        if command_array.is_empty() {
                            return Err(ParsingError::MalformedRequest);
                        }
                        let values: Result<VecDeque<Bytes>, ParsingError> = command_array
                            .into_iter()
                            .map(|value| {
                                let RespValue::BulkString(val) = value else {
                                    return Err(ParsingError::MalformedRequest);
                                };
                                Ok(Bytes::from(val))
                            })
                            .collect();
                        Ok(Command::Rpush {
                            key,
                            values: values?,
                        })
                    }
                    CommandName::Lrange => {
                        let key = parse_key(command_array.pop_front())?;
                        let start = parse_index(command_array.pop_front())?;
                        let stop = parse_index(command_array.pop_front())?;
                        if stop < start {
                            return Err(ParsingError::MalformedRequest);
                        }
                        Ok(Command::Lrange { key, start, stop })
                    }
                }
            } else {
                Err(MalformedRequest)
            }
        }
        _ => Err(MalformedRequest),
    }
}

fn parse_value(value: Option<RespValue>) -> ParsingResult<RespValue> {
    if let Some(value) = value {
        Ok(value)
    } else {
        Err(ParsingError::MalformedRequest)
    }
}

fn parse_index(value: Option<RespValue>) -> ParsingResult<i16> {
    if let Some(RespValue::BulkString(value)) = value {
        Ok(std::str::from_utf8(&value)?.parse()?)
    } else {
        Err(MalformedRequest)
    }
}

fn parse_key(key: Option<RespValue>) -> ParsingResult<Bytes> {
    if let Some(RespValue::BulkString(key)) = key {
        Ok(key)
    } else {
        Err(MalformedRequest)
    }
}

fn parse_keys(command_array: &mut VecDeque<RespValue>) -> ParsingResult<Vec<Bytes>> {
    let mut keys = Vec::with_capacity(command_array.len());
    while let Some(v) = command_array.pop_front() {
        let RespValue::BulkString(key) = v else {
            return Err(MalformedRequest);
        };
        keys.push(key);
    }
    Ok(keys)
}

fn parse_ttl(array: &mut VecDeque<RespValue>) -> ParsingResult<Option<Duration>> {
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

fn require_empty(array: VecDeque<RespValue>, command_name: CommandName) -> ParsingResult<()> {
    if !array.is_empty() {
        return Err(ParsingError::InvalidCommand(format!(
            "wrong number of arguments for '{}' command",
            command_name
        )));
    }
    Ok(())
}
