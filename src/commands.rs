/// Functions that carry out the execution of the client's CLI commands

use crate::bencode::decode_bencoded_structure;
use crate::torrent::{convert_byte_array_peers, hash_info, percent_encode_hex, print_hash_pieces,
                     TorrentInfo, TrackerRequest};
use base64::engine::general_purpose;
use base64::Engine;
use std::fs::{File};
use std::io::{Read, Write};
use std::net::{SocketAddrV4, TcpStream};
use anyhow::{Result, anyhow};

pub fn download_torrent_piece(file_name: String, output_file_name: String, piece: u32) -> Result<String> {
    match fetch_torrent_peers(file_name.clone(), false) {
        Ok(peer_array) => {
            let peer = peer_array.get(0).cloned(); // Clone the first element

            match establish_peer_connection(file_name.clone(), peer.unwrap(), false) {
                Ok((_peer_stream, peer_id)) => {
                    println!("Completed handshake with peer: {peer_id} -- Going to get piece {piece} and write to {output_file_name}");
                    Ok("Unfinished".to_string())
                }
                Err(e) => Err(anyhow!("Failed establishing connection to peer {}: {}", peer.unwrap(), e))
            }
        }
        Err(e) => Err(anyhow!("Failed getting peer array: {}", e))
    }
}

pub fn establish_peer_connection(file_name: String, peer_address: SocketAddrV4, print: bool) -> Result<(TcpStream, String)> {
    match fetch_torrent_info(file_name, false) {
        Ok(torrent_info) => {
            let mut handshake_message: Vec<u8> = Vec::new();
            let raw_hashed_info = hex::decode(torrent_info.1).expect("Can't decode hashed info from hex");
            let peer_id = "00112233445566778899";
            let protocol_name = "BitTorrent protocol";

            handshake_message.push(protocol_name.len() as u8);
            handshake_message.extend_from_slice(protocol_name.as_bytes());
            handshake_message.extend(std::iter::repeat(0).take(8));
            handshake_message.extend(raw_hashed_info);
            handshake_message.extend_from_slice(peer_id.as_bytes());


            let mut stream = TcpStream::connect(peer_address)?;
            let mut buffer = [0; 1024];

            stream.write_all(&handshake_message)?;
            let bytes_read = stream.read(&mut buffer)?;

            let (_, right) = buffer[..bytes_read].split_at(bytes_read - 20);
            let hex_encoded_peer_id = hex::encode(right);
            if print {
                println!("Peer ID: {}", hex_encoded_peer_id);
            }
            Ok((stream, hex_encoded_peer_id))
        }
        Err(e) => Err(anyhow!("Error getting torrent info {}", e))
    }
}

pub fn fetch_torrent_peers(file_name: String, print: bool) -> Result<Vec<SocketAddrV4>> {
    if let Ok(TorrentInfo(announce_url, hashed_info, length)) = fetch_torrent_info(file_name, false) {
        let percent_encoded = percent_encode_hex(hashed_info);
        let tracker_request = TrackerRequest {
            info_hash: percent_encoded.to_string(),
            peer_id: "00112233445566778899".to_string(),
            port: 6881,
            uploaded: 0,
            downloaded: 0,
            left: length as u32,
            compact: 1,
        };

        let url_with_query = format!("{}?{}", announce_url, tracker_request.to_query_string());
        let response = reqwest::blocking::get(url_with_query).expect("Query failed");
        if response.status().is_success() {
            let body_bytes = response
                .bytes()
                .expect("Couldn't convert to bytes")
                .to_vec();
            let response_decoded = decode_bencoded_structure(body_bytes);
            match response_decoded {
                Ok(value) => {
                    let peers = value
                        .as_object()
                        .expect("Unable to convert value to object")
                        .get("peers")
                        .expect("Unable to get peers");
                    let peers_string = peers.as_str().expect("Couldn't get peer string");
                    let peers_string_decoded = general_purpose::STANDARD
                        .decode(peers_string)
                        .expect("Can't decode peers from hex");
                    let peer_byte_array = peers_string_decoded.as_slice();
                    let peer_array = convert_byte_array_peers(peer_byte_array);
                    if print {
                        for peer in &peer_array {
                            println!("{peer}");
                        }
                    }
                    Ok(peer_array)
                }
                Err(e) => {
                    Err(anyhow!("Couldn't decode response: {}", e))
                }
            }
        } else {
            Err(anyhow!("Bad response from client! {}", response.status()))
        }
    } else {
        Err(anyhow!("Was unable to fetch torrent info"))
    }
}

pub fn fetch_torrent_info(file_name: String, print: bool) -> Result<TorrentInfo> {
    let mut bytes = Vec::new();
    match File::open(file_name) {
        Ok(mut file) => {
            if let Ok(_) = file.read_to_end(&mut bytes) {
                let decoded_file_result = decode_bencoded_structure(bytes);
                match decoded_file_result {
                    Ok(decoded_file) => {
                        let file_obj = decoded_file
                            .as_object()
                            .expect("Unable to convert file to object");

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

                        if print {
                            println!("Tracker URL: {}", announce);
                            println!("Length: {}", length);
                            println!("Info Hash: {}", hashed_info);
                            print_hash_pieces(info);
                        }

                        Ok(TorrentInfo(announce.to_string(), hashed_info, length))
                    }
                    Err(e) => Err(anyhow!("Wasn't able to decode the bencoded file: {}", e)),
                }
            } else {
                Err(anyhow!("Failed to read file"))
            }
        }
        Err(e) => {
            Err(anyhow!("Error opening file: {}", e))
        }
    }
}

pub fn print_bencoded_string(string: String) -> () {
    let encoded_value_bytes = Vec::from(string.as_bytes());
    let decoded_value = decode_bencoded_structure(encoded_value_bytes);
    match decoded_value {
        Ok(value) => {
            println!("{}", value.to_string());
        }
        Err(_) => {
            eprintln!("Wasn't able to decode bencoded string")
        }
    }
}