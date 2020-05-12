mod handler;
mod strings;
mod terraria_pcap;

use regex::Regex;
use serde::Deserialize;
use serenity::client::Client;
use serenity::framework::standard::StandardFramework;
use serenity::http::Http;
use serenity::model::id::ChannelId;
use std::error::Error;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;

#[derive(Deserialize)]
struct Config {
    bot_token: String,
    server_url: Option<String>,
    bridge_channel_id: u64,
    server_logfile: String,
    tcpdump_interface: String,
}

fn main() {
    let cfg: Config =
        toml::from_str(&std::fs::read_to_string("config.toml").expect("Error reading config.toml"))
            .expect("Error parsing config.toml");

    let client_handler = handler::Handler {
        playing: cfg.server_url,
        bridge_channel_id: cfg.bridge_channel_id,
    };

    let mut client = Client::new(cfg.bot_token, client_handler).expect("Error creating client");
    client.with_framework(StandardFramework::new().configure(|c| c.prefix("!")));

    // Handle SIGINT and SIGTERM and shutdown client before killing
    let shutdown_manager = client.shard_manager.clone();
    if let Err(e) = ctrlc::set_handler(move || {
        shutdown_manager.lock().shutdown_all();
    }) {
        eprintln!("Error setting SIGINT handler: {}", e);
    }

    send_loglines(
        &cfg.server_logfile,
        client.cache_and_http.http.clone(),
        ChannelId(cfg.bridge_channel_id),
    )
    .expect("Unable to start log file thread");

    terraria_pcap::parse_packets(
        client.cache_and_http.http.clone(),
        ChannelId(cfg.bridge_channel_id),
        &cfg.tcpdump_interface,
    )
    .expect("Unable to start packet parsing thread");

    if let Err(e) = client.start() {
        eprintln!("An error occurred while running the client: {:?}", e);
    }
}

// "tail"s server logfile, sending new lines to discord
fn send_loglines(
    filename: &str,
    http: Arc<Http>,
    channel_id: ChannelId,
) -> Result<(), Box<dyn Error>> {
    let tail = Command::new("tail")
        .stdout(Stdio::piped())
        .args(&["-n", "0", "-F", filename])
        .spawn()?;

    // Look for chat messages, joins, and leaves
    let send_regex = Regex::new("(^<.+>.+$|^.+ has joined\\.$|^.+ has left\\.$)").unwrap();
    let server_regex = Regex::new("^<Server>.+$").unwrap();

    let mut reader = BufReader::new(tail.stdout.expect("Missing stdout on tail child"));

    thread::spawn(move || {
        loop {
            let mut line = String::new();
            if let Err(e) = reader.read_line(&mut line) {
                eprintln!("Error reading from tail stdout: {}", e);
                continue;
            }
            let line = line.trim();

            // If line has content and matches one of the lines we want to send to discord
            if !line.is_empty() && send_regex.is_match(line) && !server_regex.is_match(line) {
                if let Err(e) = channel_id.say(&http, line) {
                    eprintln!("Unable to send logline to discord: {}", e);
                }
            }
        }
    });

    Ok(())
}
