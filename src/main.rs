use serde_json;
use std::{env, fs};
use std::iter::Peekable;
use std::str::Chars;
use serde_json::{Map, Value};
use serde_json::Value::Array;
use serde_json::Value::Object;

fn decode_bencoded_structure_new(encoded_value: &str) -> Result<Value, &'static str> {
    let mut chars = encoded_value.chars().peekable();
    parse_bencoded_values(&mut chars)
}

fn parse_bencoded_values(chars: &mut Peekable<Chars>) -> Result<Value, &'static str> {
    let num_str: String = chars
        .clone()
        .take_while(|c| c.is_digit(10))
        .collect();

    if !num_str.is_empty() {
        if let Ok(num) = num_str.parse() {
            return parse_bencoded_string(chars, num);
        }
    }

    match chars.peek() {
        Some('i') => parse_bencoded_number(chars),
        Some('l') => parse_bencoded_list(chars),
        Some('d') => parse_bencoded_map(chars),
        _ => Err("Unable to parse bencoded values"),
    }
}

fn parse_bencoded_number(chars: &mut Peekable<Chars>) -> Result<Value, &'static str> {
    let mut number = String::new();
    let mut is_float = false;

    chars.next(); // i

    while let Some(c) = chars.next() {
        match c {
            '.' => {
                is_float = true;
                number.push(c);
            }
            'e' => {
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
            _ => number.push(c),
        }
    }
    Err("Unclosed number")
}

fn parse_bencoded_string(chars: &mut Peekable<Chars>, length: usize) -> Result<Value, &'static str> {
    while let Some(c) = chars.next() {
        match c {
            ':' => {
                let data: String = chars.take(length).collect();
                return Ok(Value::String(data));
            }
            _ => {
                continue;
            }
        }
    }

    Err("Bad string")

}

fn parse_bencoded_list(chars: &mut Peekable<Chars>) -> Result<Value, &'static str> {
    let mut list = Vec::new();

    chars.next(); // l

    while let Some(c) = chars.peek() {
        match c {
            'e' => {
                chars.next(); // e
                return Ok(Array(list))
            }
            _ => {
                list.push(parse_bencoded_values(chars)?);
            }
        }
    }
    Err("Unclosed list")
}

fn parse_bencoded_map(chars: &mut Peekable<Chars>) -> Result<Value, &'static str> {
    let mut map: Map<String, Value> = Map::new();

    chars.next(); // d

    while let Some(c) = chars.peek() {
        match c {
            'e' => {
                chars.next(); // e
                return Ok(Object(map))
            }
            _ => {
                let hopeful_key = parse_bencoded_values(chars);
                match hopeful_key {
                    Ok(value) => {
                        match value {
                            Value::String(s) => {
                                let value = parse_bencoded_values(chars);
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
        let decoded_value = decode_bencoded_structure_new(encoded_value);
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
