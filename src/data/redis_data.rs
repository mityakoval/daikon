use std::collections::VecDeque;

use bytes::Bytes;

#[derive(Debug)]
pub(crate) enum RedisData {
    String(Bytes),
    List(VecDeque<Bytes>),
}
