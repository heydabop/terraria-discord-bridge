use crate::strings;
use serenity::http::Http;
use serenity::model::id::ChannelId;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

const STRING_START: usize = 7;

#[derive(Debug)]
struct MissingDeathData {
    pub desc: String,
}

impl fmt::Display for MissingDeathData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.desc)
    }
}

impl Error for MissingDeathData {}

// read pcap file of server output looking for relevant messages
pub fn parse_packets(
    http: Arc<Http>,
    channel_id: ChannelId,
    tcpdump_interface: &str,
) -> Result<(), Box<dyn Error>> {
    let tcpdump = Command::new("tcpdump")
        .stdout(Stdio::piped())
        .args(&[
            "-i",
            tcpdump_interface,
            "tcp",
            "src",
            "port",
            "7777",
            "-w",
            "-",
        ])
        .spawn()?;

    let mut reader = pcap::Reader::new(tcpdump.stdout.expect("Missing stdout on tcpdump child"))?;

    let strings = strings::get();

    thread::spawn(move || {
        let mut last_deaths: HashMap<String, Instant> = HashMap::new();

        loop {
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
                        match get_string(&data, STRING_START) {
                            Err(e) => {
                                eprintln!("Unable to parse string: {}", e);
                                continue;
                            }
                            Ok(death_type) => {
                                // Have first string in message, should be DeathSource.xxx or DeathText.xxx
                                match if death_type.find("DeathSource.") == Some(0) {
                                    assemble_death_source(
                                        &data,
                                        &death_type,
                                        &strings,
                                        &mut last_deaths,
                                    )
                                } else if death_type.find("DeathText.") == Some(0) {
                                    assemble_death_text(
                                        &data,
                                        &death_type,
                                        &strings,
                                        &mut last_deaths,
                                    )
                                } else {
                                    Err(MissingDeathData {
                                        desc: (format!("Unknown death cause: {}", death_type)),
                                    })
                                } {
                                    Err(e) => {
                                        eprintln!("Error assembling death message: {}", e);
                                        if let Err(e) = channel_id.say(&http, death_type) {
                                            eprintln!(
                                                "Unable to send death notice to discord: {}",
                                                e
                                            );
                                        }
                                    }
                                    Ok(msg) => {
                                        if !msg.is_empty() {
                                            if let Err(e) = channel_id.say(&http, msg) {
                                                eprintln!(
                                                    "Unable to send death notice to discord: {}",
                                                    e
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        };
                    }
                },
            };
        }
    });

    Ok(())
}

// Find string starting at start, strings appear to be length followed by the string
// ex [5Death], [8Terraria]
fn get_string(packet: &[u8], start: usize) -> Result<&str, std::str::Utf8Error> {
    let length = packet[start] as usize;
    std::str::from_utf8(&packet[start + 1..=start + length])
}

// Takes a string of "s1.s2" and finds it in our hashmaps looking for strings["s1"]["s2]
// ex. "DeathSource.Player" would result in strings["DeathSource"]["Player"] -> "{0} by {1}'s {2}."
fn lookup_string<'a>(
    s: &'a str,
    strings: &HashMap<&'static str, HashMap<&'static str, &'static str>>,
) -> Option<&'static str> {
    match s.find('.') {
        None => None,
        Some(i) => {
            let s1 = &s[0..i];
            let s2 = &s[i + 1..s.len()];
            match strings.get(s1) {
                None => None,
                Some(strings) => match strings.get(s2) {
                    None => None,
                    Some(s_final) => Some(s_final),
                },
            }
        }
    }
}

fn assemble_death_source(
    packet: &[u8],
    base_lookup: &str,
    strings: &HashMap<&'static str, HashMap<&'static str, &'static str>>,
    last_deaths: &mut HashMap<String, Instant>,
) -> Result<String, MissingDeathData> {
    let mut offset = STRING_START + base_lookup.len() + 3;
    match lookup_string(base_lookup, strings) {
        None => Err(MissingDeathData {
            desc: format!("Unknown death source: {}", base_lookup),
        }),
        Some(base) => match get_string(&packet, offset) {
            Err(e) => Err(MissingDeathData {
                desc: format!(
                    "Unable to parse death message base from death source: {}",
                    e
                ),
            }),
            Ok(death_message_base) => match lookup_string(death_message_base, strings) {
                None => Err(MissingDeathData {
                    desc: format!(
                        "Unknown death message base in death source: {}",
                        death_message_base
                    ),
                }),
                Some(death_message) => {
                    offset += death_message_base.len() + 3;
                    match get_string(&packet, offset) {
                        Err(e) => Err(MissingDeathData {
                            desc: format!("Unable to parse player name from death source: {}", e),
                        }),
                        Ok(player_name) => {
                            offset += player_name.len() + 8;
                            match get_string(&packet, offset) {
                                Err(e) => Err(MissingDeathData {
                                    desc: format!(
                                        "Unable to parse second string from death source: {}",
                                        e
                                    ),
                                }),
                                Ok(second_sym) => {
                                    if base_lookup == "DeathSource.Player" {
                                        // second string in death source is player name, not lookup
                                        // get 3rd string, is a lookup
                                        offset += second_sym.len() + 2;
                                        return match get_string(&packet, offset) {
                                            Err(e) => Err(MissingDeathData {
                                                desc: format!(
                                                    "Unable to parse third string from death source: {}",
                                                    e
                                                ),
                                            }),
                                            Ok(third_sym) => match lookup_string(third_sym, strings) {
                                                None => Err(MissingDeathData {
                                                    desc: format!(
                                                        "Unable to lookup third string {} from death source",
                                                        third_sym
                                                    ),
                                                }),
                                                Some(third) => {
                                                    let now = Instant::now();
                                                    match last_deaths.get(player_name) {
                                                        None => {}
                                                        Some(&last_death) => {
                                                            if now.duration_since(last_death).as_secs() < 5 {
                                                                //repeat packet, ignore
                                                                return Ok(String::from(""));
                                                            }
                                                        }
                                                    }
                                                    last_deaths.insert(String::from(player_name), now);

                                                    Ok(base
                                                       .replacen("{0}", death_message, 1)
                                                       .replacen("{0}", player_name, 1)
                                                       .replacen("{1}", second_sym, 1)
                                                       .replacen("{2}", third, 1))
                                                }
                                            },
                                        };
                                    }
                                    match lookup_string(second_sym, strings) {
                                        None => Err(MissingDeathData {
                                            desc: format!(
                                                "Unable to lookup second string {} from death source",
                                                second_sym
                                            ),
                                        }),
                                        Some(second) => {
                                            let now = Instant::now();
                                            match last_deaths.get(player_name) {
                                                None => {}
                                                Some(&last_death) => {
                                                    if now.duration_since(last_death).as_secs() < 5 {
                                                        //repeat packet, ignore
                                                        return Ok(String::from(""));
                                                    }
                                                }
                                            }
                                            last_deaths.insert(String::from(player_name), now);

                                            Ok(base
                                               .replacen("{0}", death_message, 1)
                                               .replacen("{0}", player_name, 1)
                                               .replacen("{1}", second, 1))
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            },
        },
    }
}

fn assemble_death_text(
    packet: &[u8],
    base_lookup: &str,
    strings: &HashMap<&'static str, HashMap<&'static str, &'static str>>,
    last_deaths: &mut HashMap<String, Instant>,
) -> Result<String, MissingDeathData> {
    match lookup_string(base_lookup, strings) {
        None => Err(MissingDeathData {
            desc: format!("Unknown death text: {}", base_lookup),
        }),
        Some(base) => match get_string(&packet, STRING_START + base_lookup.len() + 3) {
            Err(e) => Err(MissingDeathData {
                desc: format!("Unable to parse player name from death text: {}", e),
            }),
            Ok(player_name) => {
                let now = Instant::now();
                match last_deaths.get(player_name) {
                    None => {}
                    Some(&last_death) => {
                        if now.duration_since(last_death).as_secs() < 5 {
                            //repeat packet, ignore
                            return Ok(String::from(""));
                        }
                    }
                }
                last_deaths.insert(String::from(player_name), now);
                Ok(base.replacen("{0}", player_name, 1))
            }
        },
    }
}
