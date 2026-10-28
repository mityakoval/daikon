use anyhow::Ok;

use crate::data::resp_types::RespValue::{self, Err};

pub(crate) fn invoke(value: RespValue) -> anyhow::Result<RespValue> {
    match value {
        RespValue::BulkString(value) => Ok(RespValue::SimpleString(value)),
        _ => Ok(Err("Malformed arguments for command 'echo'".into())),
    }
}
