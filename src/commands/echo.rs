use anyhow::Ok;

use crate::data::{
    commands::CommandArray,
    resp_types::RespValue::{self, Err},
};

pub(crate) fn invoke(command_array: CommandArray) -> anyhow::Result<RespValue> {
    match command_array.values.unwrap().into_iter().next() {
        Some(RespValue::BulkString(value)) => Ok(RespValue::SimpleString(value)),
        _ => Ok(Err("Malformed arguments for command 'echo'".into())),
    }
}
