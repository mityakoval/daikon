use std::{ops::Add, time::SystemTime};

use crate::{
    data::{
        commands::CommandArray,
        types::{
            StoredValue,
            Value::{self, SimpleString},
        },
    },
    Storage,
};

pub(crate) fn invoke(storage: &Storage, command_array: CommandArray) -> anyhow::Result<Value> {
    storage.insert(
        command_array.key.unwrap(),
        StoredValue {
            value: command_array.value.unwrap(),
            expires_at: command_array.ttl.map(|ttl| SystemTime::now().add(ttl)),
        },
    );
    Ok(SimpleString("OK".into()))
}
