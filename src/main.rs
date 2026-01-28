mod strings;
mod terraria_pcap;

use poise::serenity_prelude as serenity;
use regex::Regex;
use serde::Deserialize;
use serenity::client::Client;
use serenity::http::Http;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, Pool, Postgres};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{error, info};

struct Data {
    db: Pool<Postgres>,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Deserialize)]
struct Config {
    bot_token: String,
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

#[tokio::main]
#[allow(clippy::panic, clippy::expect_used)]
async fn main() {
    tracing_subscriber::fmt::init();

    let cfg: Config =
        toml::from_str(&fs::read_to_string("config.toml").expect("Error reading config.toml"))
            .expect("Error parsing config.toml");

    let db_options = PgConnectOptions::new()
        .host(&cfg.postgres.host)
        .port(cfg.postgres.port)
        .username(&cfg.postgres.user)
        .database(&cfg.postgres.dbname)
        .password(&cfg.postgres.pass)
        .disable_statement_logging();

    let db_pool = PgPoolOptions::new()
        .min_connections(1)
        .max_connections(4)
        .connect_with(db_options)
        .await
        .expect("Unable to connect to postgres");

    let intents = GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT;
    let data_db = db_pool.clone();
    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![deaths(), playing()],
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data { db: data_db })
            })
        })
        .build();
    let mut client = Client::builder(cfg.bot_token, intents)
        .framework(framework)
        .type_map_insert::<DbClient>(db_pool.clone())
        .await
        .expect("Error creating discord client");

    {
        let http = client.http.clone();
        let pool = db_pool.clone();
        tokio::spawn(send_loglines(
            cfg.server_logfile,
            http,
            ChannelId::new(cfg.bridge_channel_id),
            pool,
        ));
    }

    {
        let http = client.http.clone();
        let pool = db_pool.clone();
        tokio::spawn(terraria_pcap::parse_packets(
            http,
            ChannelId::new(cfg.bridge_channel_id),
            cfg.tcpdump.interface.clone(),
            cfg.tcpdump.port,
            pool,
        ));
    }

    if let Err(e) = client.start().await {
        error!(error = %e, "An error occurred while running the client");
    }
}

#[poise::command(slash_command)]
async fn deaths(ctx: Context<'_>) -> Result<(), Error> {
    let data = ctx.data();
    #[allow(clippy::expect_used)]
    let db = &data.db;

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
            if content.is_empty() {
                writeln!(content, "No deaths, yet...")?;
            }

            if let Err(e) = ctx.say(content).await {
                return Err(format!("Error replying to deaths command: {e}").into());
            }

            Ok(())
        }
    }
}

#[poise::command(slash_command)]
async fn playing(ctx: Context<'_>) -> Result<(), Error> {
    Command::new("tmux")
        .args(["send-keys", "-t", "terraria", "playing\r\n"])
        .output()
        .await?;

    ctx.say("Done").await?;

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
    #[allow(clippy::expect_used)]
    let tail = Command::new("tail")
        .stdout(Stdio::piped())
        .args(["-n", "0", "-F", &filename])
        .kill_on_drop(true)
        .spawn()
        .expect("error spawning log tail");

    // Look for chat messages, joins, and leaves
    #[allow(clippy::unwrap_used)]
    let chat_regex = Regex::new(r"^(?:: )*<(?P<user>.+?)> (?P<message>.+)$").unwrap();
    #[allow(clippy::unwrap_used)]
    let join_leave_regex =
        Regex::new(r"^(?:: )*(?P<user>\S.*) has (?P<status>joined|left)\.$").unwrap();
    #[allow(clippy::unwrap_used)]
    let playing_regex =
        Regex::new(r"^(?:: )*(?P<user>.+?) \((?:\d{1,3}\.){3}\d{1,3}:\d+\)$").unwrap();
    #[allow(clippy::unwrap_used)]
    let connected_regex = Regex::new(r"^(?:: )*(\w+ players? connected\.)$").unwrap();

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
        } else if let Some(caps) = connected_regex.captures(line)
            && let Err(e) = channel_id.say(&http, &caps[1]).await
        {
            error!(error = %e, "Unable to send playing count to discord");
        }
    }
}
