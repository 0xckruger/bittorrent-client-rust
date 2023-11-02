use serde_json;
use std::env;
use serde_json::{Number, Value};
use serde_json::Value::Array;

// Available if you need it!
// use serde_bencode

fn decode_bencoded_structure(encoded_value: &str) -> Value {
    println!("{}", encoded_value);
    if encoded_value.chars().next().unwrap() == 'l' {
        let mut i = 1;
        let mut list: Value = Array(vec![]);
        while i < encoded_value.len() - 1 {
            let value_index = decode_bencoded_value(&encoded_value[i..encoded_value.len()]);
            list.as_array_mut().expect("Not an array").push(value_index.0);
            i += value_index.1 as usize;
        }
        list
    } else {
        let value_index = decode_bencoded_value(encoded_value);
        value_index.0
    }
}
#[allow(dead_code)]
fn decode_bencoded_value(encoded_value: &str) -> (Value, i64) {
    // If encoded_value starts with a digit, it's a string
    if encoded_value.chars().next().unwrap().is_digit(10) {
        // Example: "5:hello" -> "hello"
        let colon_index = encoded_value.find(':').unwrap();
        let number_string = &encoded_value[..colon_index];
        let number = number_string.parse::<i64>().unwrap();
        let string = &encoded_value[colon_index + 1..colon_index + 1 + number as usize];
        return (Value::String(string.to_string()), number + 2);
    }
    // if encoded_value starts with an i and ends in e it's a number
    else if encoded_value.chars().next().unwrap() == 'i' {
        let e_index = encoded_value.find('e').unwrap();
        let integer_string = &encoded_value[1..e_index];
        let integer = integer_string.parse::<i64>().unwrap();
        return (Value::Number(Number::from(integer)), (integer_string.len() + 2) as i64);
    } else {
        panic!("Unhandled encoded value: {}", encoded_value)
    }
}

// Usage: your_bittorrent.sh decode "<encoded_value>"
fn main() {
    let args: Vec<String> = env::args().collect();
    let command = &args[1];

    if command == "decode" {
        // You can use print statements as follows for debugging, they'll be visible when running tests.
        //println!("Logs from your program will appear here!");

        // Uncomment this block to pass the first stage
        let encoded_value = &args[2];
        let decoded_value = decode_bencoded_structure(encoded_value);
        println!("{}", decoded_value.to_string());
    } else {
        println!("unknown command: {}", args[1])
    }
}
