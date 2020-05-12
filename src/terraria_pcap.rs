use serenity::http::Http;
use serenity::model::id::ChannelId;
use std::error::Error;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;

// read pcap file of server output looking for relevant messages
pub fn parse_packets(http: Arc<Http>, channel_id: ChannelId) -> Result<(), Box<dyn Error>> {
    let tcpdump = Command::new("tcpdump")
        .stdout(Stdio::piped())
        .args(&["-i", "lo", "tcp", "src", "port", "7777", "-w", "-"])
        .spawn()?;

    let mut reader = pcap::Reader::new(tcpdump.stdout.expect("Missing stdout on tcpdump child"))?;

    thread::spawn(move || loop {
        match reader.read_packet() {
            Err(e) => {
                eprintln!("Unable to read packet: {}", e);
                return;
            }
            Ok(packet) => match reader.data(&packet) {
                Err(e) => {
                    eprintln!("Unable to parse data from packet: {}", e);
                    continue;
                }
                Ok(data) => {
                    if data.len() < 14 {
                        continue;
                    }
                    let length = u16::from_be_bytes([data[1], data[0]]);
                    if length as usize != data.len() {
                        continue;
                    }
                    if length < 8 {
                        continue;
                    }
                    if data[2..7] != [0x52, 1, 0, 0xff, 2] {
                        // server message? in deaths and server chats, not sure of meaning
                        continue;
                    }
                    if data[8..13] != [0x44, 0x65, 0x61, 0x74, 0x68] {
                        // death messages start with "Death"
                        continue;
                    }
                    match get_string(&data, 7) {
                        Err(e) => {
                            eprintln!("Unable to parse string: {}", e);
                            continue;
                        }
                        Ok(first_string) => {
                            println!("{}", first_string);
                            if let Err(e) = channel_id.say(&http, first_string) {
                                eprintln!("Unable to send logline to discord: {}", e);
                            }
                        }
                    };
                }
            },
        };
    });

    Ok(())
}

pub fn get_string<'a>(packet: &'a [u8], start: usize) -> Result<&'a str, std::str::Utf8Error> {
    let length = packet[start] as usize;
    std::str::from_utf8(&packet[start + 1..start + length + 1])
}
