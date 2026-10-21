use std::collections::VecDeque;

use crate::data::resp_types::RESPValue;
use bytes::BytesMut;

pub fn parse_data_bytes(bytes: &mut BytesMut) -> Option<(RESPValue, BytesMut)> {
    let (chunk, mut rest) = next_resp_chunk(bytes)?;
    trace!("\nparsing chunk: {:?}, rest: {:?}", chunk, rest);
    let mut chars = chunk.iter();
    match chars.next() {
        Some(b'$') => {
            // it's a BulkString
            if let Ok(length) = str::from_utf8(chars.as_slice()).unwrap().parse::<usize>() {
                let content = str::from_utf8(rest.split_to(length).iter().as_slice())
                    .unwrap()
                    .to_string();
                trace!(
                    "got bulk string {:?} of length: {}",
                    content.as_str(),
                    length
                );
                Some((RESPValue::BulkString(content), rest.split_off(2)))
            } else {
                None
            }
        }
        Some(b'*') => {
            // it's a RESPArray
            if let Ok(length) = str::from_utf8(chars.as_slice()).unwrap().parse::<usize>() {
                trace!("got a resp array of length: {}", length);
                let mut array = VecDeque::with_capacity(length);
                let mut rest = rest;
                for i in 0..length {
                    trace!("parsing item {}", i);
                    let (t, new_rest) = parse_data_bytes(&mut rest)?;
                    array.push_back(t);
                    rest = new_rest;
                }
                Some((RESPValue::Array(array), rest))
            } else {
                None
            }
        }
        None => {
            if rest.is_empty() {
                None
            } else {
                parse_data_bytes(&mut rest)
            }
        }
        _ => None,
    }
}

fn next_resp_chunk(bytes: &mut BytesMut) -> Option<(BytesMut, BytesMut)> {
    trace!("\ngetting next chunk from: {:?}", bytes);
    let chunk_sep_start = bytes.iter().position(|&x| x == b'\r')?;
    trace!("found potential chunk separator at: {}", chunk_sep_start);
    let next_byte = bytes.get(chunk_sep_start)?;
    // that's '\n'
    if *next_byte == 13 {
        let chunk = (bytes).split_to(chunk_sep_start);
        trace!("chunk is: {:?}", chunk);
        assert_eq!(bytes.first(), Some(&b'\r'));
        let rest = bytes.split_off(2);
        trace!("rest is: {:?}", rest);
        assert_ne!(rest.first(), Some(&b'\r'));
        return Some((chunk, rest));
    }
    trace!("no more chunks found");
    None
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_next_resp_chunk() {
        let mut bytes = BytesMut::from("*1\r\n$3\r\nkey\r\n");
        let (chunk, _rest) = next_resp_chunk(&mut bytes).unwrap();
        assert_eq!(chunk.freeze().as_ref(), "*1".as_bytes())
    }
}
