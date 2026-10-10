use crate::{
    DbMutex,
    data::{commands::CommandArray, resp_types::RESPValue},
};

pub(crate) fn invoke(
    storage_mut: &DbMutex,
    command_array: CommandArray,
) -> anyhow::Result<RESPValue> {
    Ok(RESPValue::Integer(1))
}
