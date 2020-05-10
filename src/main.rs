mod handler;

use regex::Regex;
use serde::Deserialize;
use serenity::client::Client;
use serenity::framework::standard::StandardFramework;
use serenity::http::Http;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use std::error::Error;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;
use std::thread;

struct ServerInPipe;

impl TypeMapKey for ServerInPipe {
    type Value = File;
}

#[derive(Deserialize)]
struct Config {
    bot_token: String,
    server_url: Option<String>,
    bridge_channel_id: u64,
    server_logfile: String,
    server_in_pipe: String,
}

fn main() {
    let cfg: Config =
        toml::from_str(&std::fs::read_to_string("config.toml").expect("Error reading config.toml"))
            .expect("Error parsing config.toml");

    // Get handle to server stdin named pipe
    let server_in_pipe = OpenOptions::new()
        .append(true)
        .create(false)
        .open(cfg.server_in_pipe)
        .expect("Unable to open server_in_pipe");

    let client_handler = handler::Handler {
        playing: cfg.server_url,
        bridge_channel_id: cfg.bridge_channel_id,
    };

    let mut client = Client::new(cfg.bot_token, client_handler).expect("Error creating client");
    client.with_framework(StandardFramework::new().configure(|c| c.prefix("!")));

    {
        // Add server stdin named pipe to client's shared data
        let mut data = client.data.write();
        data.insert::<ServerInPipe>(server_in_pipe);
    }

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
    let mut file = File::open(filename)?;
    let mut pos = file.metadata()?.len();

    // Look for chat messages, joins, and leaves
    let send_regex = Regex::new("(^<.+>.+$|^.+ has joined\\.$|^.+ has left\\.$)").unwrap();
    let server_regex = Regex::new("^<Server>.+$").unwrap();

    thread::spawn(move || {
        loop {
            // Seek to end of file so we block on read instead of getting EOF
            file.seek(SeekFrom::Start(pos))
                .expect("Unable to seek to end of logfile");

            let mut line = String::new();
            let bytes = file
                .read_to_string(&mut line)
                .expect("Unable to read line from logfile");
            let line = line.trim();

            // If line has content and matches one of the lines we want to send to discord
            if !line.is_empty() && send_regex.is_match(line) && !server_regex.is_match(line) {
                if let Err(e) = channel_id.say(&http, line) {
                    eprintln!("Unable to send logline to discord: {}", e);
                }
            }
            // Record new offset to re-seek to end of file
            pos += bytes as u64;
        }
    });

    Ok(())
}
