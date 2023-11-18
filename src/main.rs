use serde_json;
use std::{env, fs};
use std::fs::read;
use std::iter::Peekable;
use std::str::FromStr;
use std::vec::IntoIter;
use serde_json::{Map, Value};
use serde_json::Value::Array;
use serde_json::Value::Object;
use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use sha1::{Sha1, Digest};
use std::net::{Ipv4Addr};
use reqwest;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};


fn decode_bencoded_structure(encoded_value: Vec<u8>) -> Result<Value, &'static str> {
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
    while let Some(c) = bytes.next() {
        match c {
            b':' => {
                let data: Vec<u8> = bytes.take(length).collect();
                return if let Ok(utf8_string) = std::str::from_utf8(&data) {
                    Ok(Value::String(utf8_string.parse().unwrap()))
                } else {
                    Ok(Value::String(general_purpose::STANDARD.encode(&data)))
                }
            }
            _ => {
                continue;
            }
        }
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

#[derive(Debug)]
enum Command {
    Decode,
    Info,
    Peers,
}

impl FromStr for Command {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "decode" => Ok(Command::Decode),
            "info" => Ok(Command::Info),
            "peers" => Ok(Command::Peers),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
struct Info {
    length: i64,
    name: String,
    #[serde(rename = "piece length")]
    piece_length: i64,
    #[serde(with = "serde_bytes")]
    pieces: Vec<u8>,
}

fn hash_info(info: &Map<String, Value>) -> String {
    let length = info.get("length").expect("Length not present").as_i64().unwrap();
    let name = info.get("name").expect("Name not present").as_str().unwrap().to_string();
    let piece_length = info.get("piece length").expect("Piece length not present").as_i64().unwrap();
    let pieces_base64 = info.get("pieces").expect("Pieces not present").as_str().unwrap();
    let pieces = general_purpose::STANDARD.decode(pieces_base64).expect("Can't decode pieces");

    let info_struct = Info {
        length,
        name,
        piece_length,
        pieces,
    };

    let bencoded_info = serde_bencode::to_bytes(&info_struct).expect("Couldn't bencode info");
    let mut hasher = Sha1::new();
    hasher.update(&bencoded_info);
    let result = hasher.finalize();

    format!("{:x}", result)
}

fn print_hash_pieces(info: &Map<String, Value>) -> () {
    let piece_length = info.get("piece length").expect("Piece length not present").as_i64().unwrap();
    let pieces_base64 = info.get("pieces").expect("Pieces not present").as_str().unwrap();
    let pieces = general_purpose::STANDARD.decode(pieces_base64).expect("Can't decode pieces");

    println!("Piece Length: {}", piece_length);
    println!("Piece Hashes:");
    let mut pieces_iter = pieces.chunks(20);
    while let Some(chunk) = pieces_iter.next() {
        for byte in chunk {
            print!("{:02x}", byte);
        }
        println!();
    }
}

struct TrackerRequest {
    info_hash: String,
    peer_id: String,
    port: i32,
    uploaded: u32,
    downloaded: u32,
    left: u32,
    compact: u8,
}

impl TrackerRequest {
    fn to_query_string(&self) -> String {
        format!(
            "info_hash={}&peer_id={}&port={}&uploaded={}&downloaded={}&left={}&compact={}",
            self.info_hash,
            self.peer_id,
            self.port,
            self.uploaded,
            self.downloaded,
            self.left,
            self.compact
        )
    }
}

fn generate_peer_id(length: usize) -> String {
    let rng = thread_rng();
    let random_string: String = rng.sample_iter(&Alphanumeric).take(length).map(char::from).collect();
    random_string
}

fn hex_string_to_readable(hex_string: String) -> String {
    let mut readable_string = String::new();
    let mut hex_chars = hex_string.chars();

    while let (Some(first), Some(second)) = (hex_chars.next(), hex_chars.next()) {
        let byte = u8::from_str_radix(&format!("{}{}", first, second), 16);
        if let Ok(byte_value) = byte {
            if byte_value.is_ascii() {
                match byte_value {
                    b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'-' | b'_' | b'.' | b'~' => {
                        readable_string.push(byte_value as char);
                    }
                    _ => readable_string.push_str(&format!("%{:02x}", byte_value)),
                }
            } else {
                readable_string.push_str(&format!("%{:02x}", byte_value));
            }
        } else {
            println!("Invalid hex string: {}", hex_string);
            return String::new();
        }
    }

    readable_string
}

fn tracker_url_request(tracker_url: &str, info_hash: String) -> () {
    let percent_encoded = hex_string_to_readable(info_hash);
    let tracker_request = TrackerRequest {
        info_hash: percent_encoded.to_string(),
        peer_id: generate_peer_id(20),
        port: 6881,
        uploaded: 0,
        downloaded: 0,
        left: 0,
        compact: 1,
    };

    let url_with_query = format!("{}?{}", tracker_url, tracker_request.to_query_string());
    //println!("{}", url_with_query);
    let response = reqwest::blocking::get(url_with_query).expect("Query failed");
    if response.status().is_success() {
        let body_bytes = response.bytes().expect("Couldn't convert to bytes").to_vec();
        let response_decoded = decode_bencoded_structure(body_bytes);
        match response_decoded {
            Ok(value) => {
                let peers = value.as_object().expect("Unable to convert to object").get("peers").expect("Unable to get peers");
                println!("Peers string: {:?}", peers);
                if let Ok(peers_vec) = serde_json::to_vec(peers) {
                    for chunk in peers_vec.chunks_exact(6) {
                        let ip = Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]);
                        let port = u16::from_be_bytes([chunk[4], chunk[5]]);
                        println!("{}:{}", ip, port);
                    }
                }
            }
            Err(e) => {
                eprintln!("Couldn't decode response: {}", e);
            }
        }

    } else {
        eprintln!("Bad response from client! {}", response.status());
    }
}

fn read_torrent_file(bytes: Vec<u8>, command: Command) -> () {
    let parsed_file = decode_bencoded_structure(bytes);

    match parsed_file {
        Ok(file) => {
            let file_obj = file.as_object().expect("Unable to convert file to object");

            let announce = file_obj
                .get("announce")
                .expect("Announce not found in parsed file")
                .as_str()
                .expect("Announce is not a string");

            let info = file_obj
                .get("info")
                .expect("Info not found")
                .as_object()
                .expect("Info is not an object");

            let length = info
                .get("length")
                .expect("Length not found")
                .as_i64()
                .expect("Length is not an integer");

            let hashed_info = hash_info(info);

            match command {
                Command::Info => {
                    println!("Tracker URL: {}", announce);
                    println!("Length: {}", length);
                    println!("Info Hash: {}", hashed_info);

                    print_hash_pieces(info);
                }
                Command::Peers => {
                    tracker_url_request(announce, hashed_info);
                }
                _ => {
                    eprintln!("Nothing implemented here yet!");
                }
            }
        }
        Err(_) => {
            eprintln!("Error parsing file");
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: decode [bencoded string], info [torrent file], peers [torrent file]");
        return;
    }

    let command = &args[1];

    match Command::from_str(command) {
        Ok(command) => {
            match command {
                Command::Decode => {
                    let encoded_value = &args[2];
                    let encoded_value_bytes = Vec::from(encoded_value.as_bytes());
                    let decoded_value = decode_bencoded_structure(encoded_value_bytes);
                    println!("{}", decoded_value.unwrap().to_string());
                }
                Command::Info | Command::Peers => {
                    let file_name = &args[2];
                    if fs::metadata(file_name).is_ok() {
                        if let Ok(bytes) = read(file_name) {
                            read_torrent_file(bytes, command)
                        } else {
                            panic!("Couldn't read the file")
                        }
                    } else {
                        panic!("File does not exist")
                    }
                }
            }
        }
        _ => {}
    }
}
