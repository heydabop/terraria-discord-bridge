use serenity::client::Client;
use serenity::framework::standard::{
    macros::{command, group},
    CommandResult, StandardFramework,
};
use serenity::model::channel::Message;
use serenity::prelude::{Context, EventHandler};

#[group]
#[commands(ping)]
struct Terraria;

struct Handler;

impl EventHandler for Handler {}

fn main() {
    let config = std::fs::read_to_string("config.toml")
        .expect("Error reading config.toml")
        .parse::<toml::Value>()
        .expect("Error parsing config.toml");

    let bot_token = config["bot_token"]
        .as_str()
        .expect("config.toml missing bot_token");

    let mut client = Client::new(bot_token, Handler).expect("Error creating client");
    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("/"))
            .group(&TERRARIA_GROUP),
    );

    if let Err(e) = client.start() {
        println!("An error occurred while running the client: {:?}", e);
    }
}

#[command]
fn ping(ctx: &mut Context, msg: &Message) -> CommandResult {
    msg.reply(ctx, "Pong!")?;

    Ok(())
}
