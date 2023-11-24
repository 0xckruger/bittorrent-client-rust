/// Main function, associated Command types and their entry points
mod bencode;
mod commands;
mod torrent;

use crate::commands::{print_bencoded_string, establish_peer_connection, fetch_torrent_info, fetch_torrent_peers, download_torrent_piece};
use std::str::FromStr;
use std::{env, fs};
use std::net::SocketAddrV4;

#[derive(Debug)]
pub enum Command {
    Decode(String),
    Info(String),
    Peers(String),
    Handshake { file_name: String, peer_address: SocketAddrV4 },
    DownloadPiece { file_name: String, output_file_path: String, piece: u32 }
}

impl FromStr for Command {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let args: Vec<String> = env::args().collect();

        match s.to_lowercase().as_str() {
            "decode" => {
                if args.len() < 3 {
                    return Err("Bencoded string required".to_string());
                }
                Ok(Command::Decode(args[2].clone()))
            }
            "info" | "peers" | "handshake" => {
                if s == "handshake" && args.len() < 4 {
                    return Err("File name and peer IP:port required".to_string());
                } else if args.len() < 3 {
                    return Err("File name required".to_string());
                }
                match fs::metadata(&args[2]) {
                    Ok(_) => {
                        if s == "info" {
                            Ok(Command::Info(args[2].clone()))
                        } else if s == "peers" {
                            Ok(Command::Peers(args[2].clone()))
                        } else if s == "handshake" {
                            Ok(Command::Handshake {
                                file_name: args[2].clone(),
                                peer_address: args[3].clone().parse().expect("Not a valid address!"),
                            })
                        } else {
                            Err(format!("This isn't right."))
                        }
                    }
                    Err(_) => Err(format!("File '{}' not found", &args[2])),
                }
            }
            "download_piece" => {
                if args.len() < 6 {
                    return Err("Usage: 'download_piece -o /tmp/test-piece-0 sample.torrent 0'".to_string());
                }
                match fs::metadata(&args[4]) {
                    Ok(_) => {
                        Ok(Command::DownloadPiece {
                            file_name: args[4].clone(),
                            output_file_path: args[3].clone(),
                            piece: args[5].clone().parse().unwrap(),
                        })
                    }
                    Err(_) => Err(format!("File '{}' not found", &args[4])),
                }
            }
            _ => Err("Invalid command!".to_string()),
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 || &args[2] == "help" {
        eprintln!(
            "Usage: decode [bencoded string], info [torrent file], peers [torrent file]\
        , handshake [torrent file] [peer ip: peer port]"
        );
        return;
    }

    let command = &args[1];

    match Command::from_str(command) {
        Ok(command) => match command {
            Command::Decode(bencoded_value) => {
                print_bencoded_string(bencoded_value)
            }
            Command::Info(file_name) => {
                if let Err(err) = fetch_torrent_info(file_name, true) {
                    eprintln!("Error: {}", err);
                }
            }
            Command::Peers(file_name) => {
                if let Err(err) = fetch_torrent_peers(file_name, true) {
                    eprintln!("Error: {}", err);
                }
            },
            Command::Handshake { file_name, peer_address } =>
                if let Err(err) = establish_peer_connection(file_name, peer_address, true) {
                    eprintln!("Error: {}", err);
                }
            Command::DownloadPiece { file_name, output_file_path, piece } => {
                if let Err(err) = download_torrent_piece(file_name, output_file_path, piece) {
                    eprintln!("Error: {}", err);
                }
            }
        },
        _ => {}
    }
}
