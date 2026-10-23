use std::collections::VecDeque;

use crate::data::{
    errors::ParsingError::{self, IncompleteRequest, MalformedRequest},
    resp_types::RESPValue,
};
use bytes::BytesMut;

pub fn parse_data_bytes(
    bytes: &mut BytesMut,
) -> Result<Option<(RESPValue, BytesMut)>, ParsingError> {
    let Some((chunk, mut rest)) = next_resp_chunk(bytes)? else {
        return Ok(None);
    };
    trace!("\nparsing chunk: {:?}, rest: {:?}", chunk, rest);
    let mut chars = chunk.iter();
    match chars.next() {
        Some(b'$') => {
            // it's a BulkString
            if let Ok(length) = str::from_utf8(chars.as_slice()).unwrap().parse::<usize>() {
                let content = str::from_utf8(rest.split_to(length).iter().as_slice())?.to_string();
                trace!(
                    "got bulk string {:?} of length: {}",
                    content.as_str(),
                    length
                );
                Ok(Some((RESPValue::BulkString(content), rest.split_off(2))))
            } else {
                Err(ParsingError::MalformedRequest)
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
                    let Some((t, new_rest)) = parse_data_bytes(&mut rest)? else {
                        return Err(IncompleteRequest);
                    };
                    array.push_back(t);
                    rest = new_rest;
                }
                Ok(Some((RESPValue::Array(array), rest)))
            } else {
                return Err(MalformedRequest);
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

fn next_resp_chunk(bytes: &mut BytesMut) -> Result<Option<(BytesMut, BytesMut)>, ParsingError> {
    trace!("\ngetting next chunk from: {:?}", bytes);
    let Some(chunk_sep_start) = bytes.iter().position(|&x| x == b'\r') else {
        return Ok(None);
    };
    trace!("found potential chunk separator at: {}", chunk_sep_start);
    let Some(next_byte) = bytes.get(chunk_sep_start + 1) else {
        return Err(MalformedRequest);
    };
    println!("next_byte: {}", next_byte);
    // that's '\n'
    if *next_byte != 10 {
        return Err(MalformedRequest);
    }
    let chunk = (bytes).split_to(chunk_sep_start);
    trace!("chunk is: {:?}", chunk);
    assert_eq!(bytes.first(), Some(&b'\r'));
    let rest = bytes.split_off(2);
    trace!("rest is: {:?}", rest);
    assert_ne!(rest.first(), Some(&b'\r'));
    return Ok(Some((chunk, rest)));
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_next_resp_chunk() {
        let mut bytes = BytesMut::from("*1\r\n$3\r\nkey\r\n");
        let (chunk, mut rest) = next_resp_chunk(&mut bytes).unwrap();

        assert_eq!(chunk.freeze().as_ref(), "*1".as_bytes());
        assert_eq!(rest.clone().freeze().as_ref(), "$3\r\nkey\r\n".as_bytes());

        let (chunk, rest) = next_resp_chunk(&mut rest).unwrap();
        assert_eq!(chunk.freeze().as_ref(), "$3".as_bytes());
        assert_eq!(rest.clone().freeze().as_ref(), "key\r\n".as_bytes());
    }

    #[test]
    fn test_parse_bulk_string() {
        let mut bytes = BytesMut::from("$12\r\ni'm a string\r\n");

        let (RESPValue::BulkString(value), rest) = parse_data_bytes(&mut bytes).unwrap() else {
            panic!("Expected BulkSting, got something else")
        };

        assert_eq!(value, "i'm a string".to_string());
        assert!(rest.is_empty());
    }

    #[test]
    fn test_parse_incomplete_resp_array() {
        let mut bytes = BytesMut::from(
            "*12\r\n$5\r\nrpush\r\n$3\r\nkey\r\n$1\r\n1\r\n$1\r\n2\r\n$1\r\n3\r\n$1\r\n4\r\n$1\r\n5\r\n$1\r\n",
        );

        let (RESPValue::Array(values), rest) = parse_data_bytes(&mut bytes).unwrap() else {
            panic!("Expected BulkSting, got something else")
        };

        assert_eq!(values.len(),);
        assert!(rest.is_empty());
    }
}
