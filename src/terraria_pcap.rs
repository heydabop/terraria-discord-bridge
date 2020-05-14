use crate::strings;
use postgres::types::Type;
use postgres::{Client, Statement};
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
    is_pk: bool,
}

// read pcap file of server output looking for relevant messages
pub fn parse_packets(
    http: Arc<Http>,
    channel_id: ChannelId,
    interface: &str,
    port: u16,
    mut db: Client,
) -> Result<(), Box<dyn Error>> {
    let tcpdump = Command::new("tcpdump")
        .stdout(Stdio::piped())
        .args(&[
            "-i",
            interface,
            "tcp",
            "src",
            "port",
            &port.to_string(),
            "-w",
            "-",
        ])
        .spawn()?;

    let strings = strings::get();

    let insert_death = db.prepare_typed(
        "INSERT INTO death(victim, killer, weapon, message, seconds_since_last, is_pk) VALUES ($1, $2, $3, $4, $5, $6)",
        &[Type::VARCHAR, Type::VARCHAR, Type::VARCHAR, Type::TEXT, Type::INT4, Type::BOOL],
    )?;

    thread::spawn(move || {
        let mut reader =
            pcap::Reader::new(tcpdump.stdout.expect("Missing stdout on tcpdump child"))
                .expect("Unable to start pcap reader");

        let mut last_deaths: HashMap<String, u32> = HashMap::new();

        let mut last_sends: HashMap<String, u32> = HashMap::new();

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
                        let message =
                            if length >= 12 && data[8..13] == [0x44, 0x65, 0x61, 0x74, 0x68] {
                                // death messages start with "Death"
                                try_death(
                                    data,
                                    packet.epoch_seconds(),
                                    &strings,
                                    &mut last_deaths,
                                    &mut db,
                                    &insert_death,
                                )
                            } else {
                                try_generic(data, &strings)
                            };
                        if let Some(message) = message {
                            let repeat = match last_sends.get(&message) {
                                None => false,
                                Some(last_send) => packet.epoch_seconds() - last_send < 3,
                            };
                            last_sends.insert(message.clone(), packet.epoch_seconds());
                            if !repeat {
                                if let Err(e) = channel_id.say(&http, message) {
                                    eprintln!("Unable to announce to discord: {}", e);
                                }
                            }
                        }

                        continue;
                    }
                },
            }
        }
    });

    Ok(())
}

fn try_death(
    data: &[u8],
    epoch_seconds: u32,
    strings: &HashMap<&'static str, HashMap<&'static str, &'static str>>,
    last_deaths: &mut HashMap<String, u32>,
    db: &mut Client,
    insert_death: &Statement,
) -> Option<String> {
    match build_death(&data[STRING_START..data.len()], &strings) {
        Err(e) => {
            eprintln!("Error building death message: {}", e);
            None
        }
        Ok(death) => {
            let last_death = last_deaths.get(&death.victim);

            let seconds_since_last = if let Some(&last_death) = last_death {
                let seconds = epoch_seconds - last_death;
                if seconds < 5 {
                    //repeat packet, ignore
                    return None;
                }
                Some(seconds)
            } else {
                None
            };

            if let Err(e) = db.execute(
                insert_death,
                &[
                    &death.victim,
                    &death.killer,
                    &death.weapon,
                    &death.msg,
                    &(if let Some(seconds) = seconds_since_last {
                        #[allow(clippy::cast_possible_wrap)]
                        Some(seconds as i32) //still allows 68 years as an i32
                    } else {
                        None
                    }),
                    &death.is_pk,
                ],
            ) {
                eprintln!("Error inserting death: {}", e);
            }

            last_deaths.insert(death.victim, epoch_seconds);

            let message = match seconds_since_last {
                None => death.msg,
                Some(seconds) => format!(
                    "{}  *({} since last death)*",
                    death.msg,
                    friendly_duration(seconds)
                ),
            };

            Some(message)
        }
    }
}

fn friendly_duration(secs: u32) -> String {
    if secs < 120 {
        format!("{} seconds", secs)
    } else if secs < 7200 {
        format!("{:.0} minutes", (f64::from(secs) / 60.0).round())
    } else {
        format!("{:.1} hours", f64::from(secs) / 3600.0)
    }
}

// Find string starting at start, strings appear to be length followed by the string
// ex [5Death], [8Terraria]
// may also be followed by number of args in string (if string isnt an arg itself)
fn get_string(packet: &[u8], start: usize) -> Result<&str, std::str::Utf8Error> {
    let length = packet[start] as usize;
    std::str::from_utf8(&packet[start + 1..=start + length])
}

// Takes a string of "s1.s2" and finds it in our hashmaps looking for strings["s1"]["s2]
// ex. "DeathSource.Player" would result in strings["DeathSource"]["Player"] -> "{0} by {1}'s {2}."
fn lookup_string<'a>(
    s: &'a str,
    strings: &HashMap<&'static str, HashMap<&'static str, &'static str>>,
) -> Option<&'a str> {
    match s.find('.') {
        None => {
            eprintln!("Missing . in lookup: {}", s);
            None
        }
        Some(i) => {
            let s1 = &s[0..i];
            let s2 = &s[i + 1..s.len()];
            match strings.get(s1) {
                None => {
                    eprintln!("Unable to lookup first half of {}", s);
                    None
                }
                Some(strings) => match strings.get(s2) {
                    None => {
                        eprintln!("Unable to lookup second half of {}", s);
                        None
                    }
                    Some(s_final) => Some(s_final),
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
    let base = lookup_string(base_lookup, strings).unwrap_or(base_lookup);
    match get_string(&data, 0) {
        Err(e) => Err(MissingDeathData {
            desc: format!(
                "Unable to parse death message base from death source: {}",
                e
            ),
        }),
        Ok(death_message_base) => {
            let death_message =
                lookup_string(death_message_base, strings).unwrap_or(death_message_base);
            // have deathsource and deathtext, moving on to params for string substitution
            let data = &data[death_message_base.len() + 2..data.len()];
            let is_pk = base_lookup == "DeathSource.Player";
            let num_params = if is_pk { 4 } else { 3 };

            match get_params(data, &strings, num_params) {
                Err(e) => Err(MissingDeathData {
                    desc: format!("Error getting death source params: {}", e),
                }),
                Ok(params) => {
                    // Most death texts consist of something like "{0} was..."
                    // However at least one has multiple params like "{0} was removed from {1}"
                    // Every death source packet has the world name as the second param and its used here for {1}, so try to replace it if it's there
                    let death_message_subbed = death_message
                        .replacen("{0}", &format!("**{}**", params[0]), 1)
                        .replacen("{1}", params[1], 1);
                    // The base here is pretty simple, either "{0} by {1}" or "{0} by {1}'s {2}" for player kills
                    // {0} here is the death_message_subbed we just made, 1 is the killer, and 2 is the player's weapon if applicable
                    let mut final_message = base
                        .replacen("{0}", &death_message_subbed, 1)
                        .replacen("{1}", &format!("**{}**", params[2]), 1);
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
                        is_pk,
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
            params.push(lookup_string(s, strings).unwrap_or(s));
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
    let base = lookup_string(base_lookup, strings).unwrap_or(base_lookup);
    match get_string(&data, 0) {
        Err(e) => Err(MissingDeathData {
            desc: format!("Unable to parse player name after death text: {}", e),
        }),
        Ok(player_name) => Ok(Death {
            msg: base.replacen("{0}", &format!("**{}**", player_name), 1),
            victim: player_name.to_string(),
            killer: None,
            weapon: None,
            is_pk: false,
        }),
    }
}

fn try_generic(
    data: &[u8],
    strings: &HashMap<&'static str, HashMap<&'static str, &'static str>>,
) -> Option<String> {
    let mut offset = STRING_START;
    match get_string(&data, offset) {
        Err(e) => {
            eprintln!("Error parsing first generic string: {}\n{:?}", e, data);
            None
        }
        Ok(base_lookup) => {
            offset += base_lookup.len() + 1;
            if base_lookup.find("CLI.") == Some(0)
                || base_lookup == "Game.JoinGreeting"
                || base_lookup.find("LegacyMultiplayer.") == Some(0)
            {
                // Only LegacyMultipler messages were interested in are 19 and 20 (has joined) and (has left)
                return None;
            }
            println!("generic: {:?}", data);
            match lookup_string(base_lookup, strings) {
                None => {
                    eprintln!("Unable to find generic string: {}", base_lookup);
                    None
                }
                Some(base) => {
                    let mut base = base.to_string();

                    let num_params = data[offset];
                    offset += 1;
                    match get_params(&data[offset..data.len()], strings, num_params) {
                        Err(e) => {
                            eprintln!("Error getting generic params for string: {}\n{:?}", e, data);
                            None
                        }
                        Ok(params) => {
                            for (i, p) in params.iter().enumerate() {
                                println!("replace: {{{}}} {}", i, p);
                                base = base.replacen(&format!("{{{}}}", i), p, 1);
                            }
                            Some(base)
                        }
                    }
                }
            }
        }
    }
}
