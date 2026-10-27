use crate::data::commands::{ArgShape, Kommand};
use crate::data::commands::{
    Command, CommandArray,
    CommandName::{self},
};
use crate::data::errors::ParsingError::{self, InvalidCommand, MalformedRequest};
use crate::data::resp_types::RespValue;
use crate::parser::input::parse_data_bytes;
use bytes::BytesMut;
use std::collections::VecDeque;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn parse_command_array(input: &BytesMut) -> Result<CommandArray, ParsingError> {
    match parse_data_bytes(input.iter().as_slice())?.unwrap() {
        (RespValue::Array(mut command_array), _) => {
            trace!("Parsed array: {:?}", command_array);

            if let Some(RespValue::BulkString(command_bulk_str)) = command_array.pop_front() {
                let command_name =
                    CommandName::from_str(&command_bulk_str.to_uppercase().as_str())?;
                let command = match command_name {
                    CommandName::Ping => {
                        if !command_array.is_empty() {
                            eprintln!(
                                "⚠️ Expected 0 arguments for command {:?}, got {}",
                                command_name,
                                command_array.len()
                            );
                            return Err(ParsingError::InvalidCommand(
                                "wrong number of arguments for '{}' command".to_string(),
                            ));
                        }
                        Ok(Kommand::Ping)
                    }
                    CommandName::Echo => {
                        if command_array.len() == 1 {
                            let value = command_array.pop_back().unwrap();
                            Ok(Kommand::Echo(value))
                        } else {
                            eprintln!(
                                "⚠️ Expected 1 argument for command {:?}, got {}",
                                command_name,
                                command_array.len()
                            );
                             Err(InvalidCommand(format!(
                                "wrong number of arguments for '{}' command",
                                command_name
                            )))
                        }
                    },
                    CommandName::Set => {
                        if command_array.len() == 2 {
                            let (_, ttl) = parse_ttl(&mut command_array)?;
                            let key = parse_key(command_array.pop_front().unwrap())?;
                            let value = command_array.pop_back().unwrap();
                            Ok(Kommand::Set { key, value, ttl: ttl.expect("ttl must be here")  })
                        } else {
                            eprintln!(
                                "⚠️ Expected 2 arguments for command {:?}, got {}",
                                command_name,
                                command_array.len()
                            );
                            Err(InvalidCommand(format!(
                                "wrong number of arguments for '{}' command",
                                command_name
                            )))
                        }
                    },
                    CommandName::Get => {
                        
                    },
                    CommandName::Del => todo!(),
                    CommandName::Rpush => todo!(),
                }

                let command = Command::from_str(command_bulk_str.to_uppercase().as_str())?;
                let mut keys = None;
                let mut values = None;
                let mut ttl = None;

                if command.rules.allows_ttl {
                    let (remaining_array, _ttl) = parse_ttl(command_array)?;
                    ttl = _ttl;
                    command_array = remaining_array;
                }

                match command.rules.args {
                    ArgShape::None => {
                        if !command_array.is_empty() {
                            eprintln!(
                                "⚠️ Expected 0 arguments for command {:?}, got {}",
                                command.name,
                                command_array.len()
                            );
                            return Err(InvalidCommand(format!(
                                "wrong number of arguments for '{}' command",
                                command.name
                            )));
                        }
                    }
                    ArgShape::Value => {
                        if command_array.len() == 1 {
                            values = Some(VecDeque::from([command_array.pop_back().unwrap()]));
                        } else {
                            eprintln!(
                                "⚠️ Expected 1 argument for command {:?}, got {}",
                                command.name,
                                command_array.len()
                            );
                            return Err(InvalidCommand(format!(
                                "wrong number of arguments for '{}' command",
                                command.name
                            )));
                        }
                    }
                    ArgShape::Key => {
                        if command_array.len() == 1 {
                            keys = Some(vec![parse_key(command_array.pop_back().unwrap())?]);
                        } else {
                            eprintln!(
                                "⚠️ Expected 1 argument for command {:?}, got {}",
                                command.name,
                                command_array.len()
                            );
                            return Err(InvalidCommand(format!(
                                "wrong number of arguments for '{}' command",
                                command.name
                            )));
                        }
                    }
                    ArgShape::KeyValue => {
                        if command_array.len() == 2 {
                            keys = Some(vec![parse_key(command_array.pop_front().unwrap())?]);
                            values = Some(VecDeque::from([command_array.pop_back().unwrap()]));
                        } else {
                            eprintln!(
                                "⚠️ Expected 2 arguments for command {:?}, got {}",
                                command.name,
                                command_array.len()
                            );
                            return Err(InvalidCommand(format!(
                                "wrong number of arguments for '{}' command",
                                command.name
                            )));
                        }
                    }
                    ArgShape::MultipleKeys => {
                        if !command_array.is_empty() {
                            let (_, _keys) = parse_keys(command_array)?;
                            keys = Some(_keys)
                        } else {
                            eprintln!(
                                "⚠️ Expected 1+ arguments for command {:?}, got {}",
                                command.name,
                                command_array.len()
                            );
                            return Err(InvalidCommand(format!(
                                "wrong number of arguments for '{}' command",
                                command.name
                            )));
                        }
                    }
                    ArgShape::KeyMultipleValues => {
                        if command_array.len() >= 2 {
                            keys = Some(vec![parse_key(command_array.pop_front().unwrap())?]);
                            values = Some(command_array);
                        } else {
                            eprintln!(
                                "⚠️ Expected 2+ arguments for command {:?}, got {}",
                                command.name,
                                command_array.len()
                            );
                            return Err(InvalidCommand(format!(
                                "wrong number of arguments for '{}' command",
                                command.name
                            )));
                        }
                    }
                    ArgShape::Pairs => todo!(),
                }
                Ok(CommandArray {
                    command,
                    keys,
                    values,
                    ttl,
                })
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

fn parse_keys(
    mut command_array: VecDeque<RespValue>,
) -> Result<(VecDeque<RespValue>, Vec<Arc<str>>), ParsingError> {
    let mut keys = Vec::with_capacity(command_array.len());
    while let Some(v) = command_array.pop_front() {
        let RespValue::BulkString(key) = v else {
            return Err(MalformedRequest);
        };
        keys.push(Arc::<str>::from(key.to_owned()));
    }
    Ok((command_array, keys))
}

fn parse_ttl(
     array: &mut VecDeque<RespValue>,
) -> Result<(VecDeque<RespValue>, Option<Duration>), ParsingError> {
    let rem_len = array.len();
    if rem_len < 2 {
        return Ok((array, None));
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
                            "EX" => Ok((array, Some(Duration::from_secs(ttl)))),
                            "PX" => Ok((array, Some(Duration::from_millis(ttl)))),
                            _ => Err(MalformedRequest),
                        }
                    }
                    _ => Err(MalformedRequest),
                }
            } else {
                Ok((array, None))
            }
        }
        _ => Ok((array, None)),
    }
}

fn require_empty(array: VecDeque<RespValue>) -> Result<(), ParsingError> {}
