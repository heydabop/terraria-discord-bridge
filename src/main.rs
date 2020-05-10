mod handler;

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
    };

    let mut client = Client::new(bot_token, client_handler).expect("Error creating client");
    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("!"))
            .group(&TERRARIA_GROUP),
    );

    if let Err(e) = client.start() {
        eprintln!("An error occurred while running the client: {:?}", e);
    }
}
