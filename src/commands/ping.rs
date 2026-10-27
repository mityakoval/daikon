use crate::data::resp_types::RespValue;

pub(crate) fn invoke() -> RespValue {
    RespValue::SimpleString("PONG".into())
}
