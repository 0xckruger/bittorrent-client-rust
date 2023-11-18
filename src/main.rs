mod bencode;
mod torrent;

use std::{env, fs};
use std::fs::read;
use std::str::FromStr;
use crate::bencode::decode_bencoded_structure;
use crate::torrent::read_torrent_file;


#[derive(Debug)]
pub enum Command {
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
