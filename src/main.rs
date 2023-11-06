use serde_json;
use std::{env, fs};
use std::iter::Peekable;
use std::str::Chars;
use std::vec::IntoIter;
use serde_json::{Map, Value};
use serde_json::Value::Array;
use serde_json::Value::Object;

fn decode_bencoded_structure_new(encoded_value: Vec<u8>) -> Result<Value, &'static str> {
    let mut bytes = encoded_value.into_iter().peekable();
    parse_bencoded_values(&mut bytes)
}

fn parse_bencoded_values(bytes: &mut Peekable<IntoIter<u8>>) -> Result<Value, &'static str> {
    let num_str: String = bytes
        .clone()
        .take_while(|c| (b'0'..=b'9').contains(&c))
        .map(|c| c as char)
        .collect();

    if !num_str.is_empty() {
        if let Ok(num) = num_str.parse::<i64>() {
            return parse_bencoded_string(bytes, num as usize);
        }
    }

    match bytes.peek() {
        Some(&b'i') => parse_bencoded_number(bytes),
        Some(&b'l') => parse_bencoded_list(bytes),
        Some(&b'd') => parse_bencoded_map(bytes),
        _ => Err("Unable to parse bencoded values"),
    }
}

fn parse_bencoded_number(bytes: &mut Peekable<IntoIter<u8>>) -> Result<Value, &'static str> {
    let mut number = String::new();
    let mut is_float = false;

    bytes.next(); // i

    while let Some(c) = bytes.next() {
        match c {
            b'.' => {
                is_float = true;
                number.push(c as char);
            }
            b'e' => {
                if is_float {
                    if let Ok(float_val) = number.parse::<f64>() {
                        return Ok(Value::Number(serde_json::Number::from_f64(float_val).unwrap()));
                    }
                } else {
                    if let Ok(int_val) = number.parse::<i64>() {
                        return Ok(Value::Number(serde_json::Number::from(int_val)));
                    }
                }
                return Err("Invalid number");
            }
            _ => number.push(c as char),
        }
    }
    Err("Unclosed number")
}

fn parse_bencoded_string(bytes: &mut Peekable<IntoIter<u8>>, length: usize) -> Result<Value, &'static str> {
    let data: Vec<u8> = bytes.clone().take(length).collect();
    if data.len() == length {
        let string_data = String::from_utf8(data.clone());

        return match string_data {
            Ok(s) => Ok(Value::String(s)),
            Err(_) => Ok(Value::from(data)),
        };
    }
    Err("Bad string")
}

fn parse_bencoded_list(bytes: &mut Peekable<IntoIter<u8>>) -> Result<Value, &'static str> {
    let mut list = Vec::new();

    bytes.next(); // l

    while let Some(c) = bytes.peek() {
        match c {
            b'e' => {
                bytes.next(); // e
                return Ok(Array(list))
            }
            _ => {
                list.push(parse_bencoded_values(bytes)?);
            }
        }
    }
    Err("Unclosed list")
}

fn parse_bencoded_map(bytes: &mut Peekable<IntoIter<u8>>) -> Result<Value, &'static str> {
    let mut map: Map<String, Value> = Map::new();

    bytes.next(); // d

    while let Some(c) = bytes.peek() {
        match c {
            b'e' => {
                bytes.next(); // e
                return Ok(Object(map))
            }
            _ => {
                let hopeful_key = parse_bencoded_values(bytes);
                match hopeful_key {
                    Ok(value) => {
                        match value {
                            Value::String(s) => {
                                let value = parse_bencoded_values(bytes);
                                map.insert(s, value.unwrap());
                            }
                            _ => {
                                return Err("Key wasn't a string!");
                            }
                        }
                    }
                    _ => {
                        return Err("Key parsing error occurred");
                    }
                }
            }
        }
    }
    Err("Unclosed map")
}

fn read_torrent_file(bytes: Vec<u8>) -> Result<Value, &'static str> {
    unimplemented!("Not done yet");
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        let encoded_value = &args[2];
        let encoded_value_bytes = Vec::from(encoded_value.as_bytes());
        let decoded_value = decode_bencoded_structure_new(encoded_value_bytes);
        println!("{}", decoded_value.unwrap().to_string());
    } else if command == "info" {
        let file_name = &args[2];
        if fs::metadata(file_name).is_ok() {
            if let Ok(bytes) = fs::read(file_name) {
                read_torrent_file(bytes).expect("Couldn't parse torrent file");
            } else {
                panic!("Couldn't read the file")
            }
        } else {
            panic!("File does not exist")
        }
    } else {
        panic!("unsupported action")
    }
}
