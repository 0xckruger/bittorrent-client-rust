/// Main function, associated Command types and their entry points
mod bencode;
mod commands;
mod torrent;

use crate::commands::{print_bencoded_string, establish_peer_connection, fetch_and_print_torrent_info, fetch_and_print_torrent_peers};
use std::fs::File;
use std::str::FromStr;
use std::{env};

#[derive(Debug)]
pub enum Command {
    Decode(String),
    Info(File),
    Peers(File),
    Handshake { file: File, string: String },
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
                match File::open(&args[2]) {
                    Ok(file) => {
                        if s == "info" {
                            Ok(Command::Info(file))
                        } else if s == "peers" {
                            Ok(Command::Peers(file))
                        } else {
                            Ok(Command::Handshake {
                                file,
                                string: args[3].clone(),
                            })
                        }
                    }
                    Err(_) => Err(format!("File '{}' not found", &args[2])),
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
            Command::Info(mut file) => {
                if let Err(err) = fetch_and_print_torrent_info(&mut file, true) {
                    eprintln!("Error: {}", err);
                }
            }
            Command::Peers(mut file) => {
                if let Err(err) = fetch_and_print_torrent_peers(&mut file) {
                    eprintln!("Error: {}", err);
                }
            },
            Command::Handshake { mut file, string } =>
                if let Err(err) = establish_peer_connection(&mut file, string) {
                    eprintln!("Error: {}", err);
                }
        },
        _ => {}
    }
}
