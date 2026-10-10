use crate::data::resp_types::RESPValue;

pub(crate) fn invoke() -> RESPValue {
    RESPValue::SimpleString("PONG".into())
}
