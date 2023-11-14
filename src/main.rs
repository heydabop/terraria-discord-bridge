mod handler;
mod strings;
mod terraria_pcap;

use regex::Regex;
use serde::Deserialize;
use serenity::client::Client;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandResult, StandardFramework};
use serenity::http::Http;
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, Pool, Postgres};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{error, info};

#[derive(Deserialize)]
struct Config {
    bot_token: String,
    server_url: Option<String>,
    bridge_channel_id: u64,
    server_logfile: String,
    postgres: PgConfig,
    tcpdump: TcpDumpConfig,
}

#[derive(Deserialize)]
struct PgConfig {
    host: String,
    port: u16,
    user: String,
    pass: String,
    dbname: String,
}

#[derive(Deserialize)]
struct TcpDumpConfig {
    interface: String,
    port: u16,
}

pub struct DbClient;

impl TypeMapKey for DbClient {
    type Value = Pool<Postgres>;
}

#[group]
#[commands(deaths, playing)]
struct Terraria;

#[tokio::main]
#[allow(clippy::panic, clippy::expect_used)]
async fn main() {
    tracing_subscriber::fmt::init();

    let cfg: Config =
        toml::from_str(&std::fs::read_to_string("config.toml").expect("Error reading config.toml"))
            .expect("Error parsing config.toml");

    let mut db_options = PgConnectOptions::new()
        .host(&cfg.postgres.host)
        .port(cfg.postgres.port)
        .username(&cfg.postgres.user)
        .database(&cfg.postgres.dbname)
        .password(&cfg.postgres.pass);
    db_options.disable_statement_logging();

    let db_pool = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(4)
        .connect_with(db_options)
        .await
        .expect("Unable to connect to postgres");

    let client_handler = handler::Handler {
        playing: cfg.server_url,
        bridge_channel_id: cfg.bridge_channel_id,
    };

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    let mut client = Client::builder(cfg.bot_token, intents)
        .framework(
            StandardFramework::new()
                .configure(|c| c.prefix("/").allow_dm(false).case_insensitivity(true))
                .group(&TERRARIA_GROUP),
        )
        .type_map_insert::<DbClient>(db_pool.clone())
        .event_handler(client_handler)
        .await
        .expect("Error creating discord client");

    {
        let http = client.cache_and_http.http.clone();
        let pool = db_pool.clone();
        tokio::spawn(send_loglines(
            cfg.server_logfile.clone(),
            http,
            ChannelId(cfg.bridge_channel_id),
            pool,
        ));
    }

    {
        let http = client.cache_and_http.http.clone();
        let pool = db_pool.clone();
        tokio::spawn(terraria_pcap::parse_packets(
            http,
            ChannelId(cfg.bridge_channel_id),
            cfg.tcpdump.interface.clone(),
            cfg.tcpdump.port,
            pool,
        ));
    }

    if let Err(e) = client.start().await {
        error!(error = %e, "An error occurred while running the client");
    }
}

#[command]
async fn deaths(ctx: &Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read().await;
    #[allow(clippy::expect_used)]
    let db = data
        .get::<crate::DbClient>()
        .expect("Failed to get database pool from context");

    #[allow(clippy::panic)]
    match sqlx::query!("SELECT victim FROM death").fetch_all(db).await {
        Err(e) => Err(format!("Unable to query deaths: {e}").into()),
        Ok(rows) => {
            let mut death_map: HashMap<String, u32> = HashMap::new();
            for row in rows {
                let victim: String = row.victim;
                match death_map.get(&victim) {
                    None => death_map.insert(victim, 1),
                    Some(&victim_deaths) => death_map.insert(victim, victim_deaths + 1),
                };
            }
            let mut deaths = vec![];
            for (victim, victim_deaths) in &death_map {
                deaths.push((victim, victim_deaths));
            }
            deaths.sort_by(|a, b| b.1.cmp(a.1));

            let mut content = String::new();
            for death in deaths {
                writeln!(content, "{} - {}", death.0, death.1)?;
            }

            if let Err(e) = msg.channel_id.say(&ctx.http, content).await {
                return Err(format!("Error replying to deaths command: {e}").into());
            }

            Ok(())
        }
    }
}

#[command]
async fn playing(_ctx: &Context, _msg: &Message) -> CommandResult {
    Command::new("tmux")
        .args(["send-keys", "-t", "terraria", "playing\r\n"])
        .output()
        .await?;

    Ok(())
}

// "tail"s server logfile, sending new lines to discord
async fn send_loglines(
    filename: String,
    http: Arc<Http>,
    channel_id: ChannelId,
    db: Pool<Postgres>,
) {
    #[allow(clippy::expect_used)]
    let tail = Command::new("tail")
        .stdout(Stdio::piped())
        .args(["-n", "0", "-F", &filename])
        .kill_on_drop(true)
        .spawn()
        .expect("error spawning log tail");

    // Look for chat messages, joins, and leaves
    #[allow(clippy::unwrap_used)]
    let chat_regex = Regex::new("^(?:: )*<(?P<user>.+?)> (?P<message>.+)$").unwrap();
    #[allow(clippy::unwrap_used)]
    let join_leave_regex =
        Regex::new("^(?:: )*(?P<user>\\S.*) has (?P<status>joined|left)\\.$").unwrap();
    #[allow(clippy::unwrap_used)]
    let playing_regex =
        Regex::new("^(?:: )*(?P<user>.+?) \\((?:\\d{1,3}\\.){3}\\d{1,3}:\\d+\\)$").unwrap();
    #[allow(clippy::unwrap_used)]
    let connected_regex = Regex::new("^(?:: )*(\\w+ players? connected\\.)$").unwrap();

    #[allow(clippy::expect_used)]
    let mut reader = BufReader::new(tail.stdout.expect("Missing stdout on tail child"));

    info!("starting log reader loop");
    loop {
        let mut line = String::new();
        if let Err(e) = reader.read_line(&mut line).await {
            error!(error = %e, "Error reading from tail stdout");
            continue;
        }
        let line = line.trim();

        // If line has content and matches one of the lines we want to send to discord
        if let Some(caps) = chat_regex.captures(line) {
            let user = &caps["user"];
            if user != "Server" {
                let message = &caps["message"];
                if let Err(e) = channel_id
                    .say(&http, format!("<{}> {}", &caps["user"], &caps["message"]))
                    .await
                {
                    error!(error = %e, "Unable to send chat to discord");
                }
                #[allow(clippy::panic)]
                if let Err(e) = sqlx::query!(
                    r#"INSERT INTO message(author, content) VALUES ($1, $2)"#,
                    user,
                    message
                )
                .execute(&db)
                .await
                {
                    error!(error = %e, "Unable to insert terraria message into db");
                }
            }
        } else if let Some(caps) = join_leave_regex.captures(line) {
            let user = &caps["user"];
            let status = &caps["status"];
            if let Err(e) = channel_id
                .say(&http, format!("{} has {}", &caps["user"], &caps["status"]))
                .await
            {
                error!(error = %e, "Unable to send chat to discord");
            }
            #[allow(clippy::panic)]
            if let Err(e) = match status {
                "joined" => sqlx::query!(r#"INSERT INTO server_join(username) VALUES ($1)"#, user)
                    .execute(&db)
                    .await
                    .map(|_| ()),
                "left" => sqlx::query!(r#"INSERT INTO server_leave(username) VALUES ($1)"#, user)
                    .execute(&db)
                    .await
                    .map(|_| ()),
                _ => Ok(()),
            } {
                error!(error = %e, "Error inserting terraria user status");
            }
        } else if let Some(caps) = playing_regex.captures(line) {
            if let Err(e) = channel_id.say(&http, &caps["user"]).await {
                error!(error = %e, "Unable to send playing to discord");
            }
        } else if let Some(caps) = connected_regex.captures(line) {
            if let Err(e) = channel_id.say(&http, &caps[1]).await {
                error!(error = %e, "Unable to send playing count to discord");
            }
        }
    }
}
