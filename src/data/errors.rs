use bytes::BytesMut;
use crate::data::types::RESPType;

pub struct Error<'a> {
    msg: &'a str,
}

impl<'a> RESPType for Error<'a> {
    fn encode(&mut self) -> BytesMut {
        BytesMut::from(self.msg)
    }
}