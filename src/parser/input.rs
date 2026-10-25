use std::collections::VecDeque;

use crate::data::{
    errors::ParsingError::{self, IncompleteRequest, MalformedRequest},
    resp_types::RESPValue,
};

pub fn parse_data_bytes(bytes: &[u8]) -> Result<Option<(RESPValue, &[u8])>, ParsingError> {
    let Some((sep_start, next_chunk_at)) = next_resp_separator(bytes)? else {
        return Ok(None);
    };
    let chunk = &bytes[..sep_start];

    if next_chunk_at >= bytes.len() {
        return Err(ParsingError::IncompleteRequest);
    }

    let mut rest = &bytes[next_chunk_at..];
    trace!("rest!!: {:?}", str::from_utf8(rest));
    trace!("\nparsing chunk: {:?}", str::from_utf8(chunk));
    let mut chars = chunk.iter();
    match chars.next() {
        Some(b'$') => {
            // it's a BulkString
            if let Ok(length) = str::from_utf8(chars.as_slice()).unwrap().parse::<usize>() {
                if rest.len() < length {
                    return Err(ParsingError::IncompleteRequest);
                }

                let potential_chunk = &rest[..length];

                if potential_chunk.contains(&b'\r') {
                    trace!(
                        "chunk contains a separator: {:?}",
                        str::from_utf8(potential_chunk)
                    );
                    return Err(ParsingError::MalformedRequest);
                }

                let content = str::from_utf8(potential_chunk)?.to_string();
                trace!(
                    "got bulk string {:?} of length: {}",
                    content.as_str(),
                    length
                );
                Ok(Some((
                    RESPValue::BulkString(content),
                    &rest[(length + 2)..],
                )))
            } else {
                Err(ParsingError::MalformedRequest)
            }
        }
        Some(b'*') => {
            // it's a RESPArray
            if let Ok(length) = str::from_utf8(chars.as_slice()).unwrap().parse::<usize>() {
                trace!("got a resp array of length: {}", length);

                let mut array = VecDeque::with_capacity(length);

                for i in 0..length {
                    trace!("parsing item {}", i);
                    let Some((t, new_rest)) = parse_data_bytes(rest)? else {
                        return Err(IncompleteRequest);
                    };
                    trace!("push {:?} to command array", t);
                    array.push_back(t);
                    rest = new_rest;
                    trace!("new rest: {:?}", str::from_utf8(rest));
                }
                Ok(Some((RESPValue::Array(array), rest)))
            } else {
                Err(MalformedRequest)
            }
        }
        None => {
            if rest.is_empty() {
                Ok(None)
            } else {
                Err(ParsingError::IncompleteRequest)
            }
        }
        _ => Ok(None),
    }
}

fn next_resp_separator(bytes: &[u8]) -> Result<Option<(usize, usize)>, ParsingError> {
    trace!("\ngetting next chunk from: {:?}", str::from_utf8(bytes));
    let Some(sep_start) = bytes.iter().position(|&x| x == b'\r') else {
        return Ok(None);
    };
    trace!("found potential chunk separator at: {}", sep_start);
    let Some(next_byte) = bytes.get(sep_start + 1) else {
        return Err(MalformedRequest);
    };

    // that's '\n'
    if *next_byte != 10 {
        return Err(MalformedRequest);
    }
    let next_chunk_start = sep_start + 2;
    let remaining_buf = &bytes[next_chunk_start..];
    trace!("next chunk is at pos: {:?}", next_chunk_start);

    assert_ne!(remaining_buf.first(), Some(&b'\r'));

    Ok(Some((sep_start, next_chunk_start)))
}

#[cfg(test)]
mod tests {

    use std::assert_matches;

    use super::*;

    #[test]
    fn test_next_resp_chunk() {
        let mut carret: usize = 0;
        let bytes = "*1\r\n$3\r\nkey\r\n".as_bytes();
        let (sep_1_start, chunk_1_at) = next_resp_separator(bytes)
            .expect("expected separator boundaries")
            .unwrap();

        assert_eq!(sep_1_start, 2);
        assert_eq!(chunk_1_at, 4);
        carret += chunk_1_at;

        let (sep_start, next_chunk_at) = next_resp_separator(&bytes[carret..])
            .expect("expected separator boundaries")
            .unwrap();

        assert_eq!(sep_start, 2);
        assert_eq!(next_chunk_at, 4);
        carret += next_chunk_at;

        let (sep_start, next_chunk_at) = next_resp_separator(&bytes[carret..])
            .expect("expected separator boundaries")
            .unwrap();

        assert_eq!(sep_start, 3);
        assert_eq!(next_chunk_at, 5);
    }

    #[test]
    fn test_parse_bulk_string() {
        let bytes = "$12\r\ni'm a string\r\n".as_bytes();

        let (RESPValue::BulkString(value), rest) = parse_data_bytes(bytes)
            .expect("expected next chunk")
            .unwrap()
        else {
            panic!("Expected BulkSting, got something else")
        };

        assert_eq!(value, "i'm a string".to_string());
        assert!(rest.is_empty());
    }

    #[test]
    fn test_parse_incomplete_resp_array() {
        let  bytes = "*12\r\n$5\r\nrpush\r\n$3\r\nkey\r\n$1\r\n1\r\n$1\r\n2\r\n$1\r\n3\r\n$1\r\n4\r\n$1\r\n5\r\n$1\r\n".as_bytes();
        let result = parse_data_bytes(bytes).expect_err("expected ParsingError::IncompleteRequest");

        assert_matches!(result, ParsingError::IncompleteRequest);
    }

    #[test]
    fn test_parse_resp_array() {
        let  bytes = "*8\r\n$5\r\nrpush\r\n$3\r\nkey\r\n$1\r\n1\r\n$1\r\n2\r\n$1\r\n3\r\n$1\r\n4\r\n$1\r\n5\r\n$1\r\n6\r\n".as_bytes();
        let (value, _) = parse_data_bytes(bytes)
            .expect("expected Ok result")
            .unwrap();

        let RESPValue::Array(vec) = value else {
            panic!("Extected RESPArray")
        };

        assert_eq!(vec.len(), 8);
        assert_eq!(vec[0], RESPValue::BulkString("rpush".to_string()));
    }
}
