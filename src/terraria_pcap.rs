use crate::strings;
use serenity::http::Http;
use serenity::model::id::ChannelId;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;

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

struct Death {
    msg: String,
    victim: String,
    killer: Option<String>,
    weapon: Option<String>,
}

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
        let mut last_deaths: HashMap<String, u32> = HashMap::new();

        loop {
            match reader.read_packet() {
                Err(e) => {
                    eprintln!("Unable to read packet: {}", e);
                    return;
                }
                Ok(packet) => match reader.data(&packet.bytes()) {
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
                        match build_death(&data[STRING_START..data.len()], &strings) {
                            Err(e) => eprintln!("Error building death message: {}", e),
                            Ok(death) => {
                                match last_deaths.get(&death.victim) {
                                    None => {}
                                    Some(&last_death) => {
                                        if packet.epoch_seconds() - last_death < 5 {
                                            //repeat packet, ignore
                                            continue;
                                        }
                                    }
                                }
                                last_deaths.insert(death.victim, packet.epoch_seconds());

                                // TODO: save to DB

                                if let Err(e) = channel_id.say(&http, death.msg) {
                                    eprintln!("Unable to send death notice to discord: {}", e);
                                }
                            }
                        }
                    }
                },
            }
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
) -> &'a str {
    match s.find('.') {
        None => {
            eprintln!("Missing . in lookup: {}", s);
            s
        }
        Some(i) => {
            let s1 = &s[0..i];
            let s2 = &s[i + 1..s.len()];
            match strings.get(s1) {
                None => {
                    eprintln!("Unable to lookup first half of {}", s);
                    s
                }
                Some(strings) => match strings.get(s2) {
                    None => {
                        eprintln!("Unable to lookup second half of {}", s);
                        s
                    }
                    Some(s_final) => s_final,
                },
            }
        }
    }
}

fn build_death(
    data: &[u8],
    strings: &HashMap<&'static str, HashMap<&'static str, &'static str>>,
) -> Result<Death, MissingDeathData> {
    match get_string(&data, 0) {
        Err(e) => Err(MissingDeathData {
            desc: format!("Unable to parse first death string: {}", e),
        }),
        Ok(death_type) => {
            let data = &data[death_type.len() + 3..data.len()];
            // Have first string in message, should be DeathSource.xxx or DeathText.xxx
            if death_type.find("DeathSource.") == Some(0) {
                build_from_death_source(&data, &death_type, &strings)
            } else if death_type.find("DeathText.") == Some(0) {
                build_from_death_text(&data, &death_type, &strings)
            } else {
                Err(MissingDeathData {
                    desc: (format!("Unknown death cause: {}", death_type)),
                })
            }
        }
    }
}

fn build_from_death_source(
    data: &[u8],
    base_lookup: &str,
    strings: &HashMap<&'static str, HashMap<&'static str, &'static str>>,
) -> Result<Death, MissingDeathData> {
    let base = lookup_string(base_lookup, strings);
    match get_string(&data, 0) {
        Err(e) => Err(MissingDeathData {
            desc: format!(
                "Unable to parse death message base from death source: {}",
                e
            ),
        }),
        Ok(death_message_base) => {
            let death_message = lookup_string(death_message_base, strings);
            // have deathsource and deathtext, moving on to params for string substitution
            let data = &data[death_message_base.len() + 2..data.len()];
            let num_params = if base_lookup == "DeathSource.Player" {
                4
            } else {
                3
            };

            match get_params(data, &strings, num_params) {
                Err(e) => Err(MissingDeathData {
                    desc: format!("Error getting death source params: {}", e),
                }),
                Ok(params) => {
                    // Most death texts consist of something like "{0} was..."
                    // However at least one has multiple params like "{0} was removed from {1}"
                    // Every death source packet has the world name as the second param and its used here for {1}, so try to replace it if it's there
                    let death_message_subbed = death_message
                        .replacen("{0}", params[0], 1)
                        .replacen("{1}", params[1], 1);
                    // The base here is pretty simple, either "{0} by {1}" or "{0} by {1}'s {2}" for player kills
                    // {0} here is the death_message_subbed we just made, 1 is the killer, and 2 is the player's weapon if applicable
                    let mut final_message = base
                        .replacen("{0}", &death_message_subbed, 1)
                        .replacen("{1}", params[2], 1);
                    if num_params == 4 {
                        final_message = final_message.replacen("{2}", params[3], 1);
                    }
                    Ok(Death {
                        msg: final_message,
                        victim: params[0].to_string(),
                        killer: Some(params[2].to_string()),
                        weapon: if num_params == 4 {
                            Some(params[3].to_string())
                        } else {
                            None
                        },
                    })
                }
            }
        }
    }
}

fn get_params<'a>(
    data: &'a [u8],
    strings: &HashMap<&'static str, HashMap<&'static str, &'static str>>,
    n: u8,
) -> Result<Vec<&'a str>, std::str::Utf8Error> {
    let mut offset = 0;
    let mut params = vec![];
    for _ in 0..n {
        let s = get_string(&data, offset + 1)?;
        if data[offset] == 0 {
            // no lookup
            params.push(s);
        } else {
            params.push(lookup_string(s, strings));
        }
        offset += s.len() + 2;
    }

    Ok(params)
}

fn build_from_death_text(
    data: &[u8],
    base_lookup: &str,
    strings: &HashMap<&'static str, HashMap<&'static str, &'static str>>,
) -> Result<Death, MissingDeathData> {
    let base = lookup_string(base_lookup, strings);
    match get_string(&data, 0) {
        Err(e) => Err(MissingDeathData {
            desc: format!("Unable to parse player name after death text: {}", e),
        }),
        Ok(player_name) => Ok(Death {
            msg: base.replacen("{0}", player_name, 1),
            victim: player_name.to_string(),
            killer: None,
            weapon: None,
        }),
    }
}
