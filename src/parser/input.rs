use bytes::BytesMut;
use crate::data::types::Value;

pub fn parse_data_bytes(bytes: &mut BytesMut) -> Option<(Value, BytesMut)> {
    let (chunk, mut rest) = next_resp_chunk(bytes)?;
    eprintln!("\nparsing chunk: {:?}, rest: {:?}", chunk, rest);
    let mut chars = chunk.iter();
    match chars.next() {
        Some(b'$') => {
            // it's a BulkString
            if let Ok(length) = str::from_utf8(chars.as_slice()).unwrap().parse::<usize>() {

                let content = str::from_utf8(rest.split_to(length).iter().as_slice()).unwrap().to_string();
                eprintln!(
                    "got bulk string {:?} of length: {}",
                    content.as_str(),
                    length
                );
                Some((Value::BulkString(content), rest.split_off(2)))
            } else {
                None
            }
        }
        Some(b'*') => {
            // it's a RESPArray
            if let Ok(length) = str::from_utf8(chars.as_slice()).unwrap().parse::<usize>() {
                eprintln!("got a resp array of length: {}", length);
                let mut array = Vec::with_capacity(length);
                let mut rest = rest;
                for i in 0..length {
                    eprintln!("parsing item {}", i);
                    let (t, new_rest) = parse_data_bytes(&mut rest)?;
                    array.push(t);
                    rest = new_rest;
                }
                Some((Value::Array(array), rest))
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
        },
        _ => None
    }
}

fn next_resp_chunk(bytes: &mut BytesMut) -> Option<(BytesMut, BytesMut)> {
    eprintln!("\ngetting next chunk from: {:?}", bytes);
    let chunk_sep_start = bytes.iter().position(|&x| x == b'\r')?;
    eprintln!("found potential chunk separator at: {}", chunk_sep_start);
    let next_byte = bytes.get(chunk_sep_start)?;
    if *next_byte == 13 { // that's '\n'
        let chunk = (bytes).split_to(chunk_sep_start);
        eprintln!("chunk is: {:?}", chunk);
        assert_eq!(bytes.get(0), Some(&b'\r'));
        let rest = bytes.split_off(2);
        eprintln!("rest is: {:?}", rest);
        assert_ne!(rest.get(0), Some(&b'\r'));
        return Some((chunk, rest));
    }
    eprintln!("no more chunks found");
    None
}
