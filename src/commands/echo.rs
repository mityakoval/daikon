use anyhow::Ok;

use crate::data::{
    commands::CommandArray,
    resp_types::RESPValue::{self, Err},
};

pub(crate) fn invoke(command_array: CommandArray) -> anyhow::Result<RESPValue> {
    match command_array.values.unwrap().into_iter().next() {
        Some(RESPValue::BulkString(value)) => Ok(RESPValue::SimpleString(value)),
        _ => Ok(Err("Malformed arguments for command 'echo'".into())),
    }
}
