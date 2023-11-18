use crate::bencode::decode_bencoded_structure;
use crate::Command;
use serde_json::{Map, Value};
use sha1::{Sha1, Digest};
use serde::{Deserialize, Serialize};
use base64::{engine::general_purpose, Engine as _};
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;


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
    left: u64,
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

#[allow(dead_code)]
fn generate_peer_id(length: usize) -> String {
    let rng = thread_rng();
    let random_string: String = rng.sample_iter(&Alphanumeric).take(length).map(char::from).collect();
    random_string
}

fn percent_encode_hex(hex_string: String) -> String {
    let mut percent_encoded_string = String::new();
    let mut hex_chars = hex_string.chars();

    while let (Some(first), Some(second)) = (hex_chars.next(), hex_chars.next()) {
        let byte = u8::from_str_radix(&format!("{}{}", first, second), 16);
        if let Ok(byte_value) = byte {
            if byte_value.is_ascii() {
                match byte_value {
                    b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'-' | b'_' | b'.' | b'~' => {
                        percent_encoded_string.push(byte_value as char);
                    }
                    _ => percent_encoded_string.push_str(&format!("%{:02x}", byte_value)),
                }
            } else {
                percent_encoded_string.push_str(&format!("%{:02x}", byte_value));
            }
        } else {
            println!("Invalid hex string: {}", hex_string);
            return String::new();
        }
    }
    percent_encoded_string
}

fn print_byte_array_peers(bytes: &[u8]) -> () {
    for group in bytes.chunks(6) {
        print!("{}.{}.{}.{}", group[0], group[1], group[2], group[3]);
        let port_bytes: &[u8] = &group[4..=5]; // Slice representing the 2-byte value

        if let Ok(port_array) = port_bytes.try_into() {
            let port_value = u16::from_be_bytes(port_array);
            println!(":{}", port_value);
        } else {
            eprintln!("Invalid slice length");
        }
    }
}

fn tracker_url_request(tracker_url: &str, info_hash: String, length: u64) -> () {
    let percent_encoded = percent_encode_hex(info_hash);
    let tracker_request = TrackerRequest {
        info_hash: percent_encoded.to_string(),
        peer_id: "00112233445566778899".to_string(),
        port: 6881,
        uploaded: 0,
        downloaded: 0,
        left: length,
        compact: 1,
    };

    let url_with_query = format!("{}?{}", tracker_url, tracker_request.to_query_string());
    println!("URL: {}", url_with_query);
    let response = reqwest::blocking::get(url_with_query).expect("Query failed");
    if response.status().is_success() {
        let body_bytes = response.bytes().expect("Couldn't convert to bytes").to_vec();
        let response_decoded = decode_bencoded_structure(body_bytes);
        match response_decoded {
            Ok(value) => {
                println!("Object response: {:?}", value.as_object().unwrap());
                let peers = value.as_object()
                    .expect("Unable to convert value to object")
                    .get("peers")
                    .expect("Unable to get peers");
                println!("Peer string: {}", peers.as_str().unwrap());
                let peer_bytes = peers
                    .as_str()
                    .expect("Couldn't convert peers to string").as_bytes();
                print_byte_array_peers(peer_bytes);
            }
            Err(e) => {
                eprintln!("Couldn't decode response: {}", e);
            }
        }
    } else {
        eprintln!("Bad response from client! {}", response.status());
    }
}

pub fn read_torrent_file(bytes: Vec<u8>, command: Command) -> () {
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
                    tracker_url_request(announce, hashed_info, length as u64);
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