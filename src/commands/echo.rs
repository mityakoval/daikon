use crate::data::{
    errors::StorageResult,
    resp_types::RespValue::{self, Err},
};

pub(crate) fn invoke(value: RespValue) -> StorageResult<RespValue> {
    match value {
        RespValue::BulkString(value) => Ok(RespValue::SimpleString(value)),
        _ => Ok(Err("Malformed arguments for command 'echo'".into())),
    }
}
