mod strings;
mod terraria_pcap;

use poise::serenity_prelude as serenity;
use regex::Regex;
use serde::Deserialize;
use serenity::all::UserId;
use serenity::client::Client;
use serenity::http::Http;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use sqlx::{ConnectOptions, Pool, Postgres};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::fs;
use std::process::{Stdio, exit};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::signal::unix::{SignalKind, signal};
use tokio::time::{Duration, sleep};
use tracing::{error, info};

struct Data {
    db: Pool<Postgres>,
    admin_user_id: UserId,
    server_dir: String,
}
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

#[derive(Deserialize)]
struct Config {
    bot_token: String,
    bridge_channel_id: u64,
    admin_user_id: u64,
    server_dir: String,
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

    let mut sigint = match signal(SignalKind::interrupt()) {
        Ok(s) => s,
        Err(e) => {
            error!(%e, "Error registering SIGINT handler");
            exit(1);
        }
    };
    let mut sigterm = match signal(SignalKind::terminate()) {
        Ok(s) => s,
        Err(e) => {
            error!(%e, "Error registering SIGTERM handler");
            exit(1);
        }
    };

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
            commands: vec![deaths(), playing(), update(), version()],
            ..Default::default()
        })
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {
                    db: data_db,
                    admin_user_id: UserId::from(cfg.admin_user_id),
                    server_dir: cfg.server_dir,
                })
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

    let shard_manager = client.shard_manager.clone();
    tokio::spawn(async move {
        tokio::select! {
            _ = sigint.recv() => {},
            _ = sigterm.recv() => {},
        };
        shard_manager.shutdown_all().await;
    });

    if let Err(e) = client.start().await {
        error!(error = %e, "An error occurred while running the client");
    }
}

/// Show players sorted by how many times they've died
#[poise::command(slash_command, prefix_command)]
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

/// Show who's currently online
#[poise::command(slash_command, prefix_command)]
async fn playing(ctx: Context<'_>) -> Result<(), Error> {
    if !Command::new("tmux")
        .args(["send-keys", "-t", "terraria", "playing", "Enter"])
        .output()
        .await?
        .status
        .success()
    {
        ctx.say("Command failed").await?;
        return Ok(());
    }

    ctx.say("Sending...").await?;

    Ok(())
}

/// Show current server version
#[poise::command(slash_command, prefix_command)]
async fn version(ctx: Context<'_>) -> Result<(), Error> {
    if !Command::new("tmux")
        .args(["send-keys", "-t", "terraria", "version", "Enter"])
        .output()
        .await?
        .status
        .success()
    {
        ctx.say("Command failed").await?;
        return Ok(());
    }

    ctx.say("Sending...").await?;

    Ok(())
}

/// Update server from old version to new version
#[poise::command(slash_command, prefix_command)]
async fn update(
    ctx: Context<'_>,
    #[description = "Current server version"] old_version: String,
    #[description = "New version to update to"] new_version: String,
) -> Result<(), Error> {
    // only allow admin to invocate command
    let author_id = ctx.author().id;
    let admin_user_id = ctx.data().admin_user_id;
    if author_id != admin_user_id {
        ctx.say(format!("I only listen to <@{admin_user_id}>"))
            .await?;
        return Ok(());
    }

    ctx.defer().await?;

    let server_dir = ctx.data().server_dir.clone();
    let zipfile = format!("terraria-server-{new_version}.zip");

    // download new zip
    if !Command::new("curl")
        .current_dir(&server_dir)
        .args([
            "-o",
            &zipfile,
            &format!("https://terraria.org/api/download/pc-dedicated-server/{zipfile}",),
        ])
        .output()
        .await?
        .status
        .success()
    {
        ctx.say("Unable to download server zip").await?;
        return Ok(());
    }

    // unzip new server
    if !Command::new("unzip")
        .current_dir(&server_dir)
        .args([zipfile])
        .output()
        .await?
        .status
        .success()
    {
        ctx.say("Unable to unzip server").await?;
        return Ok(());
    }

    // chmod new binary
    if !Command::new("chmod")
        .current_dir(&server_dir)
        .args([
            String::from("u+x"),
            format!("{new_version}/Linux/TerrariaServer.bin.x86_64"),
        ])
        .output()
        .await?
        .status
        .success()
    {
        ctx.say("Unable to chmod server binary").await?;
        return Ok(());
    }

    // check if server is running
    if Command::new("pgrep")
        .args(["-f", "TerrariaServer.bin.x86_64"])
        .output()
        .await?
        .status
        .success()
    {
        // exit server if so
        Command::new("tmux")
            .args(["send-keys", "-t", "terraria", "exit", "Enter"])
            .output()
            .await?;

        sleep(Duration::from_secs(5)).await;

        // check if server is still running
        if Command::new("pgrep")
            .args(["-f", "TerrariaServer.bin.x86_64"])
            .output()
            .await?
            .status
            .success()
        {
            ctx.say("Unable to stop server").await?;
            return Ok(());
        }
    }

    // replace directory in start_server.sh
    if !Command::new("sed")
        .current_dir(&server_dir)
        .args([
            "-i",
            &format!("s/{old_version}/{new_version}/g"),
            "start_server.sh",
        ])
        .output()
        .await?
        .status
        .success()
    {
        ctx.say("Unable to sed server script").await?;
        return Ok(());
    }

    // restart server
    Command::new("tmux")
        .args(["send-keys", "-t", "terraria", "./start_server.sh", "Enter"])
        .output()
        .await?;

    sleep(Duration::from_secs(10)).await;

    // check version
    Command::new("tmux")
        .args(["send-keys", "-t", "terraria", "version", "Enter"])
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
    #[allow(clippy::unwrap_used)]
    let version_regex = Regex::new(r"^(?:: )*Terraria Server (v[0-9.]+)$").unwrap();

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
        } else if let Some(caps) = version_regex.captures(line)
            && let Err(e) = channel_id.say(&http, &caps[1]).await
        {
            error!(error = %e, "Unable to send version to discord");
        }
    }
}
