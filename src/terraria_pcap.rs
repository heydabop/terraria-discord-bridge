use crate::strings;
use serenity::http::Http;
use serenity::model::id::ChannelId;
use sqlx::{Pool, Postgres};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::process::{Command, Stdio};
use std::sync::Arc;
use tracing::{error, info};

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
pub async fn parse_packets(
    http: Arc<Http>,
    channel_id: ChannelId,
    interface: String,
    port: u16,
    db: Pool<Postgres>,
) {
    #[allow(clippy::expect_used)]
    let tcpdump = Command::new("tcpdump")
        .stdout(Stdio::piped())
        .args(&[
            "-i",
            &interface,
            "tcp",
            "src",
            "port",
            &port.to_string(),
            "-w",
            "-",
        ])
        .spawn()
        .expect("error spawning tcpdump process");

    let strings = strings::get();

    #[allow(clippy::expect_used)]
    let mut reader = pcap::Reader::new(tcpdump.stdout.expect("Missing stdout on tcpdump child"))
        .expect("Unable to start pcap reader");

    let mut last_sends: HashMap<String, u32> = HashMap::new();

    info!("starting packet reader loop");
    loop {
        let packet = match reader.read_packet() {
            Ok(p) => p,
            Err(e) => {
                error!(error = %e, "Unable to read packet");
                return;
            }
        };
        let data = match reader.data(packet.bytes()) {
            Ok(d) => d,
            Err(e) => {
                error!(error = %e, "Unable to parse data from packet");
                continue;
            }
        };
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
        let message = if length >= 12 && data[8..13] == [0x44, 0x65, 0x61, 0x74, 0x68] {
            // death messages start with "Death"
            try_death(data, &strings, &db).await
        } else {
            match try_generic(&data[6..], &strings) {
                None => None,
                Some(s) => Some(s.0),
            }
        };
        if let Some(message) = message {
            let repeat = match last_sends.get(&message) {
                None => false,
                Some(last_send) => packet.epoch_seconds() - last_send < 5,
            };
            last_sends.insert(message.clone(), packet.epoch_seconds());
            if !repeat {
                if let Err(e) = channel_id.say(&http, message).await {
                    error!(error = %e, "Unable to announce to discord");
                }
            }
        }
    }
}

async fn try_death(
    data: &[u8],
    strings: &HashMap<&'static str, HashMap<&'static str, &'static str>>,
    db: &Pool<Postgres>,
) -> Option<String> {
    match build_death(&data[STRING_START..], strings) {
        Err(e) => {
            error!(error = %e, "Error building death message");
            None
        }
        Ok(death) => {
            #[allow(clippy::panic)]
            let seconds_since_last: Option<i32> = match sqlx::query!(
                "SELECT max(create_date) as last_date FROM death WHERE victim = $1",
                death.victim
            )
            .fetch_one(db)
            .await
            {
                Ok(r) => match r.last_date {
                    Some(last_date) => {
                        let now = sqlx::types::chrono::Local::now();
                        let since = now.signed_duration_since(last_date);
                        if since.num_seconds() < 9 {
                            //respawn timer is 10s, this is a repeat packet/message
                            return None;
                        }
                        match since.num_seconds().try_into() {
                            Ok(s) => Some(s),
                            Err(e) => {
                                error!(error = %e, "error converting seconds since death");
                                None
                            }
                        }
                    }
                    None => None,
                },
                Err(e) => {
                    error!(error = %e, "error getting last death");
                    None
                }
            };

            #[allow(clippy::panic)]
            if let Err(e) = sqlx::query!(r#"INSERT INTO death(victim, killer, weapon, message, seconds_since_last, is_pk) VALUES ($1, $2, $3, $4, $5, $6)"#,
                                         death.victim, death.killer, death.weapon, death.msg, seconds_since_last, death.is_pk).execute(db).await {
                error!(error = %e, "Error inserting death");
            }

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

fn friendly_duration(secs: i32) -> String {
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
            error!(string = s, "Missing . in lookup");
            None
        }
        Some(i) => {
            let s1 = &s[0..i];
            let s2 = &s[i + 1..];
            match strings.get(s1) {
                None => {
                    error!(string = s, "Unable to lookup first half");
                    None
                }
                Some(strings) => match strings.get(s2) {
                    None => {
                        error!(string = s, "Unable to lookup second half");
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
    match get_string(data, 0) {
        Err(e) => Err(MissingDeathData {
            desc: format!("Unable to parse first death string: {}", e),
        }),
        Ok(death_type) => {
            let data = &data[death_type.len() + 3..];
            // Have first string in message, should be DeathSource.xxx or DeathText.xxx
            if death_type.find("DeathSource.") == Some(0) {
                build_from_death_source(data, death_type, strings)
            } else if death_type.find("DeathText.") == Some(0) {
                build_from_death_text(data, death_type, strings)
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
    match get_string(data, 0) {
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
            let data = &data[death_message_base.len() + 2..];
            let is_pk = base_lookup == "DeathSource.Player";
            let num_params = if is_pk { 4 } else { 3 };

            match get_params(data, strings, num_params) {
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
        let s = get_string(data, offset + 1)?;
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
    match get_string(data, 0) {
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

// strings are [mode][length][string...][num_substitutions (only if mode != 0)]
// ex. [0x2, 0x17, Announcement.HasArrived, 0x1]
// ex. [0x0, 0x8, username]
// mode is 0 for literal and 2 for localization key
// a string with a non-0 mode will be follwed by a single byte indicating how many substitutions are needed
// a string with mode 0 has no trailing byte (the next byte after a mode 0 string is the next string's mode (if there's another)
// we can recursively assemble a string by assembling all of its substitutions (and the substitutions' substitutions, etc) and then subbing them in
// TODO: this should probably also be used for death message deserialization but i still have some special functionality to work around there
fn try_generic(
    data: &[u8],
    strings: &HashMap<&'static str, HashMap<&'static str, &'static str>>,
) -> Option<(String, usize)> {
    let mut offset = 0;
    let mode = data[offset];
    offset += 1;
    match get_string(data, offset) {
        Err(e) => {
            error!(error = %e, ?data, "Error parsing first generic string");
            None
        }
        Ok(key) => {
            offset += key.len() + 1;
            if mode == 0 {
                return Some((key.to_string(), offset));
            }

            if key.find("CLI.") == Some(0)
                || key == "Game.JoinGreeting"
                || key.find("LegacyMultiplayer.") == Some(0)
            {
                return None;
            }

            info!(?data, "generic");
            let num_subs = data[offset] as usize;
            offset += 1;
            let mut subs = Vec::with_capacity(num_subs);
            for _ in 0..num_subs {
                let sub = match try_generic(&data[offset..], strings) {
                    None => return None,
                    Some(sub) => sub,
                };
                subs.push(sub.0);
                offset += sub.1;
            }
            let mut val = match lookup_string(key, strings) {
                None => return None,
                Some(val) => val.to_string(),
            };
            for (i, p) in subs.iter().enumerate() {
                info!("replace: {{{}}} {}", i, p);
                val = val.replacen(&format!("{{{}}}", i), p, 1);
            }
            Some((val, offset))
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn try_generic() {
        use super::strings;

        let strings = strings::get();

        let flesh = vec![
            0x02, 0x16, 0x41, 0x6e, 0x6e, 0x6f, 0x75, 0x6e, 0x63, 0x65, 0x6d, 0x65, 0x6e, 0x74,
            0x2e, 0x48, 0x61, 0x73, 0x41, 0x77, 0x6f, 0x6b, 0x65, 0x6e, 0x01, 0x02, 0x13, 0x4e,
            0x50, 0x43, 0x4e, 0x61, 0x6d, 0x65, 0x2e, 0x57, 0x61, 0x6c, 0x6c, 0x6f, 0x66, 0x46,
            0x6c, 0x65, 0x73, 0x68, 0x00, 0xaf, 0x4b, 0xff,
        ];
        assert_eq!(
            Some(("Wall of Flesh has awoken!".to_string(), 47)),
            super::try_generic(&flesh, &strings)
        );

        let eclipse = vec![
            0x02, 0x0d, 0x4c, 0x65, 0x67, 0x61, 0x63, 0x79, 0x4d, 0x69, 0x73, 0x63, 0x2e, 0x32,
            0x30, 0x00, 0x32, 0xff, 0x82,
        ];
        assert_eq!(
            Some(("A solar eclipse is happening!".to_string(), 16)),
            super::try_generic(&eclipse, &strings)
        );

        let merchant = vec![
            2, 23, 65, 110, 110, 111, 117, 110, 99, 101, 109, 101, 110, 116, 46, 72, 97, 115, 65,
            114, 114, 105, 118, 101, 100, 1, 2, 13, 71, 97, 109, 101, 46, 78, 80, 67, 84, 105, 116,
            108, 101, 2, 0, 5, 87, 105, 108, 108, 121, 2, 26, 78, 80, 67, 78, 97, 109, 101, 46, 84,
            114, 97, 118, 101, 108, 108, 105, 110, 103, 77, 101, 114, 99, 104, 97, 110, 116, 0, 50,
            125, 255,
        ];
        assert_eq!(
            Some(("Willy the Traveling Merchant has arrived!".to_string(), 78)),
            super::try_generic(&merchant, &strings)
        );

        let n_slime = vec![
            2, 34, 71, 97, 109, 101, 46, 69, 110, 101, 109, 105, 101, 115, 68, 101, 102, 101, 97,
            116, 101, 100, 66, 121, 65, 110, 110, 111, 117, 110, 99, 101, 109, 101, 110, 116, 3, 0,
            8, 104, 101, 121, 100, 97, 98, 111, 112, 0, 3, 50, 48, 48, 2, 17, 78, 80, 67, 78, 97,
            109, 101, 46, 66, 108, 117, 101, 83, 108, 105, 109, 101, 0, 250, 250, 0,
        ];
        assert_eq!(
            Some((
                "heydabop has defeated the 200th Blue Slime!".to_string(),
                72
            )),
            super::try_generic(&n_slime, &strings)
        );

        let kill = vec![
            0x02, 0x12, 0x44, 0x65, 0x61, 0x74, 0x68, 0x53, 0x6f, 0x75, 0x72, 0x63, 0x65, 0x2e,
            0x50, 0x6c, 0x61, 0x79, 0x65, 0x72, 0x03, 0x02, 0x22, 0x44, 0x65, 0x61, 0x74, 0x68,
            0x54, 0x65, 0x78, 0x74, 0x47, 0x65, 0x6e, 0x65, 0x72, 0x69, 0x63, 0x2e, 0x45, 0x6e,
            0x74, 0x72, 0x61, 0x69, 0x6c, 0x73, 0x52, 0x69, 0x70, 0x70, 0x65, 0x64, 0x4f, 0x75,
            0x74, 0x02, 0x00, 0x05, 0x62, 0x6f, 0x74, 0x74, 0x79, 0x00, 0x04, 0x74, 0x65, 0x73,
            0x74, 0x00, 0x10, 0x73, 0x70, 0x61, 0x63, 0x65, 0x20, 0x69, 0x6e, 0x20, 0x6d, 0x79,
            0x20, 0x6e, 0x61, 0x6d, 0x65, 0x02, 0x19, 0x49, 0x74, 0x65, 0x6d, 0x4e, 0x61, 0x6d,
            0x65, 0x2e, 0x43, 0x6f, 0x70, 0x70, 0x65, 0x72, 0x53, 0x68, 0x6f, 0x72, 0x74, 0x73,
            0x77, 0x6f, 0x72, 0x64, 0x00, 0xe1, 0x19, 0x19,
        ];
        assert_eq!(
            Some((
                "botty's entrails were ripped out by space in my name's Copper Shortsword."
                    .to_string(),
                117
            )),
            super::try_generic(&kill, &strings)
        );
    }
}
