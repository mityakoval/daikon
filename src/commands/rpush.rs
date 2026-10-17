use std::{cmp, collections::VecDeque};

use bytes::{Bytes, BytesMut};

use crate::{
    DbMutex,
    data::{
        commands::CommandArray,
        redis_data::{LP_MAX_SIZE, ListPack},
        resp_types::RESPValue,
    },
};

pub(crate) fn invoke(
    storage_mut: &DbMutex,
    command_array: CommandArray,
) -> anyhow::Result<RESPValue> {
    let mut values = command_array.values.unwrap();
    let r = values.len() % LP_MAX_SIZE;
    let mut packs = values.len() / LP_MAX_SIZE;
    if r != 0 {
        packs += 1;
    }
    for _ in 0..packs {
        let until = cmp::min(values.len(), LP_MAX_SIZE);
        let mut pack = BytesMut::new();
        for v in values.drain(..until) {}
    }
    let mut list: VecDeque<ListPack> = VecDeque::with_capacity(packs);
    Ok(RESPValue::Integer(1))
}
