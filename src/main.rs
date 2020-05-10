mod handler;

extern crate ctrlc;

use serenity::client::Client;
use serenity::framework::standard::{macros::group, StandardFramework};
use serenity::http::Http;
use serenity::model::id::ChannelId;
use std::error::Error;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::sync::Arc;
use std::thread;

#[group]
struct Terraria;

fn main() {
    let config = std::fs::read_to_string("config.toml")
        .expect("Error reading config.toml")
        .parse::<toml::Value>()
        .expect("Error parsing config.toml");

    let bot_token = config["bot_token"]
        .as_str()
        .expect("config.toml missing bot_token");

    let server_logfile = config["server_logfile"]
        .as_str()
        .expect("config.toml missing server_logfile");

    let bridge_channel_id = config["bridge_channel_id"]
        .as_str()
        .expect("config.toml missing bridge_channel_id")
        .parse::<u64>()
        .expect("bridge_channel_id not u64");

    let client_handler = handler::Handler {
        playing: config["server_url"]
            .as_str()
            .or(Some(""))
            .unwrap()
            .to_string(),
        bridge_channel_id,
    };

    let mut client = Client::new(bot_token, client_handler).expect("Error creating client");
    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("!"))
            .group(&TERRARIA_GROUP),
    );

    let shutdown_manager = client.shard_manager.clone();

    if let Err(e) = ctrlc::set_handler(move || {
        shutdown_manager.lock().shutdown_all();
    }) {
        eprintln!("Error setting SIGINT handler: {}", e);
    }

    send_loglines(
        server_logfile,
        client.cache_and_http.http.clone(),
        ChannelId(bridge_channel_id),
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
