use crate::data::commands::{ArgShape, Command, CommandArray};
use crate::data::resp_types::RESPValue;
use crate::parser::input::parse_data_bytes;
use anyhow::{Context, anyhow};
use bytes::BytesMut;
use std::collections::VecDeque;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn parse_command_array(input: &mut BytesMut) -> anyhow::Result<CommandArray> {
    match parse_data_bytes(input.iter().as_slice())?.unwrap() {
        (RESPValue::Array(mut command_array), _) => {
            trace!("Parsed array: {:?}", command_array);

            if let Some(RESPValue::BulkString(command_bulk_str)) = command_array.pop_front() {
                let command = Command::from_str(command_bulk_str.to_uppercase().as_str())
                    .context("parsing command")?;

                let mut keys = None;
                let mut values = None;
                let mut ttl = None;

                if command.rules.allows_ttl {
                    let (remaining_array, _ttl) =
                        parse_ttl(command_array).context("parsing ttl")?;
                    ttl = _ttl;
                    command_array = remaining_array;
                }

                // if command.rules.has_key {
                //
                // }

                match command.rules.args {
                    ArgShape::None => {
                        if !command_array.is_empty() {
                            eprintln!(
                                "⚠️ Expected 0 arguments for command {:?}, got {}",
                                command.name,
                                command_array.len()
                            );
                            return Err(anyhow!(
                                "wrong number of arguments for '{}' command",
                                command.name
                            ));
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
                            return Err(anyhow!(
                                "wrong number of arguments for '{}' command",
                                command.name
                            ));
                        }
                    }
                    ArgShape::Key => {
                        if command_array.len() == 1 {
                            keys = Some(vec![
                                parse_key(command_array.pop_back().unwrap())
                                    .context("parsing key")?,
                            ]);
                        } else {
                            eprintln!(
                                "⚠️ Expected 1 argument for command {:?}, got {}",
                                command.name,
                                command_array.len()
                            );
                            return Err(anyhow!(
                                "wrong number of arguments for '{}' command",
                                command.name
                            ));
                        }
                    }
                    ArgShape::KeyValue => {
                        if command_array.len() == 2 {
                            keys = Some(vec![
                                parse_key(command_array.pop_front().unwrap())
                                    .context("parsing key")?,
                            ]);
                            values = Some(VecDeque::from([command_array.pop_back().unwrap()]));
                        } else {
                            eprintln!(
                                "⚠️ Expected 2 arguments for command {:?}, got {}",
                                command.name,
                                command_array.len()
                            );
                            return Err(anyhow!(
                                "wrong number of arguments for '{}' command",
                                command.name
                            ));
                        }
                    } // ValueType::List => {
                    //     value = Some(Vec::from(command_array));
                    // }
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
                            return Err(anyhow!(
                                "wrong number of arguments for '{}' command",
                                command.name
                            ));
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
                            return Err(anyhow!(
                                "wrong number of arguments for '{}' command",
                                command.name
                            ));
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
                Err(anyhow!("command array empty or malformed"))
            }
        }
        _ => Err(anyhow!("Command must be a RESP array")),
    }
}

fn parse_key(key: RESPValue) -> anyhow::Result<Arc<str>> {
    if let RESPValue::BulkString(key) = key {
        Ok(Arc::<str>::from(key.to_owned()))
    } else {
        Err(anyhow!("key must be a BulkString"))
    }
}

fn parse_keys(
    mut command_array: VecDeque<RESPValue>,
) -> anyhow::Result<(VecDeque<RESPValue>, Vec<Arc<str>>)> {
    let mut keys = Vec::with_capacity(command_array.len());
    while let Some(v) = command_array.pop_front() {
        let RESPValue::BulkString(key) = v else {
            return Err(anyhow!("key must be a BulkString"));
        };
        keys.push(Arc::<str>::from(key.to_owned()));
    }
    Ok((command_array, keys))
}

fn parse_ttl(
    mut array: VecDeque<RESPValue>,
) -> anyhow::Result<(VecDeque<RESPValue>, Option<Duration>)> {
    let rem_len = array.len();
    if rem_len < 2 {
        return Ok((array, None));
    }
    let ttl;
    let units;
    match array[rem_len - 2] {
        RESPValue::BulkString(ref _units) => {
            if ["EX", "PX"].contains(&_units.to_uppercase().as_str()) {
                ttl = array.pop_back().unwrap();
                units = array.pop_back().unwrap();
                match units {
                    RESPValue::BulkString(units) => {
                        let ttl = match ttl {
                            RESPValue::BulkString(ttl) => ttl.parse::<u64>()?,
                            _ => {
                                return Err(anyhow!("Malformed ttl"));
                            }
                        };
                        match units.to_uppercase().as_str() {
                            "EX" => Ok((array, Some(Duration::from_secs(ttl)))),
                            "PX" => Ok((array, Some(Duration::from_millis(ttl)))),
                            _ => Err(anyhow!("Malformed ttl")),
                        }
                    }
                    _ => Err(anyhow!("Malformed ttl")),
                }
            } else {
                Ok((array, None))
            }
        }
        _ => Ok((array, None)),
    }
}
