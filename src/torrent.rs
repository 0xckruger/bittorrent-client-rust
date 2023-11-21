/// Helper functions for processing torrent files

use base64::{engine::general_purpose, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use sha1::{Digest, Sha1};

pub struct TorrentInfo(pub String, pub String, pub i64);

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
struct Info {
    length: i64,
    name: String,
    #[serde(rename = "piece length")]
    piece_length: i64,
    #[serde(with = "serde_bytes")]
    pieces: Vec<u8>,
}

pub struct TrackerRequest {
    pub(crate) info_hash: String,
    pub(crate) peer_id: String,
    pub(crate) port: i32,
    pub(crate) uploaded: u32,
    pub(crate) downloaded: u32,
    pub(crate) left: u32,
    pub(crate) compact: u8,
}

impl TrackerRequest {
    pub(crate) fn to_query_string(&self) -> String {
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

pub fn hash_info(info: &Map<String, Value>) -> String {
    let length = info
        .get("length")
        .expect("Length not present")
        .as_i64()
        .unwrap();
    let name = info
        .get("name")
        .expect("Name not present")
        .as_str()
        .unwrap()
        .to_string();
    let piece_length = info
        .get("piece length")
        .expect("Piece length not present")
        .as_i64()
        .unwrap();
    let pieces_base64 = info
        .get("pieces")
        .expect("Pieces not present")
        .as_str()
        .unwrap();
    let pieces = general_purpose::STANDARD
        .decode(pieces_base64)
        .expect("Can't decode pieces");

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

pub fn print_hash_pieces(info: &Map<String, Value>) -> () {
    let piece_length = info
        .get("piece length")
        .expect("Piece length not present")
        .as_i64()
        .unwrap();
    let pieces_base64 = info
        .get("pieces")
        .expect("Pieces not present")
        .as_str()
        .unwrap();
    let pieces = general_purpose::STANDARD
        .decode(pieces_base64)
        .expect("Can't decode pieces");

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

pub fn percent_encode_hex(hex_string: String) -> String {
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

pub fn print_byte_array_peers(bytes: &[u8]) -> () {
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
