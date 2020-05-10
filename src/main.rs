mod handler;

extern crate ctrlc;

use serde::Deserialize;
use serenity::client::Client;
use serenity::framework::standard::StandardFramework;
use serenity::http::Http;
use serenity::model::id::ChannelId;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;
use std::thread;

#[derive(Deserialize)]
struct Config {
    bot_token: String,
    server_url: Option<String>,
    bridge_channel_id: u64,
    server_logfile: String,
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

fn send_loglines(
    filename: &str,
    http: Arc<Http>,
    channel_id: ChannelId,
) -> Result<(), Box<dyn Error>> {
    let mut file = File::open(filename)?;
    let mut pos = file.metadata()?.len();

    thread::spawn(move || {
        file.seek(SeekFrom::Start(pos))
            .expect("Unable to seek to end of logfile");
        loop {
            let mut line = String::new();
            let bytes = file
                .read_to_string(&mut line)
                .expect("Unable to read line from logfile");
            if !line.is_empty() {
                if let Err(e) = channel_id.say(&http, line) {
                    eprintln!("Unable to send logline to discord: {}", e);
                }
            }
            pos += bytes as u64;
            file.seek(SeekFrom::Start(pos))
                .expect("Unable to re-seek to end of logfile");
        }
    });

    Ok(())
}
