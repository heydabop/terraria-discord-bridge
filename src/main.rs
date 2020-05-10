mod handler;

extern crate ctrlc;

use serenity::client::Client;
use serenity::framework::standard::{macros::group, StandardFramework};

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

    let client_handler = handler::Handler {
        playing: config["server_url"]
            .as_str()
            .or(Some(""))
            .unwrap()
            .to_string(),
        bridge_channel_id: config["bridge_channel_id"]
            .as_str()
            .expect("config.toml missing bridge_channel_id")
            .parse::<u64>()
            .expect("bridge_channel_id not u64"),
    };

    let mut client = Client::new(bot_token, client_handler).expect("Error creating client");
    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("!"))
            .group(&TERRARIA_GROUP),
    );

    let manager = client.shard_manager.clone();

    if let Err(e) = ctrlc::set_handler(move || {
        manager.lock().shutdown_all();
    }) {
        eprintln!("Error setting SIGINT handler: {}", e);
    }

    if let Err(e) = client.start() {
        eprintln!("An error occurred while running the client: {:?}", e);
    }
}
