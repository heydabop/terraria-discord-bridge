mod handler;
mod strings;
mod terraria_pcap;

use postgres::types::Type;
use postgres::NoTls;
use r2d2_postgres::r2d2::Pool;
use r2d2_postgres::PostgresConnectionManager;
use regex::Regex;
use serde::Deserialize;
use serenity::client::Client;
use serenity::framework::standard::macros::{command, group};
use serenity::framework::standard::{CommandError, CommandResult, StandardFramework};
use serenity::http::{CacheHttp, Http};
use serenity::model::channel::Message;
use serenity::model::id::ChannelId;
use serenity::prelude::*;
use std::collections::HashMap;
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

struct DbClient;

impl TypeMapKey for DbClient {
    type Value = Pool<PostgresConnectionManager<NoTls>>;
}

#[group]
#[commands(deaths, playing)]
struct Terraria;

fn main() {
    let cfg: Config =
        toml::from_str(&std::fs::read_to_string("config.toml").expect("Error reading config.toml"))
            .expect("Error parsing config.toml");

    let mut db_config = postgres::Config::new();
    db_config
        .host(&cfg.postgres.host)
        .port(cfg.postgres.port)
        .user(&cfg.postgres.user)
        .dbname(&cfg.postgres.dbname)
        .password(&cfg.postgres.pass);
    let db_client_pcap = db_config
        .connect(NoTls)
        .expect("Unable to connect to postgres");
    let db_client_logger = db_config
        .connect(NoTls)
        .expect("Unable to connect to postgres");
    let db_pool_serenity = Pool::new(PostgresConnectionManager::new(db_config, NoTls))
        .expect("Unable to create postgres connection pool");

    let client_handler = handler::Handler {
        playing: cfg.server_url,
        bridge_channel_id: cfg.bridge_channel_id,
    };

    let mut client = Client::new(cfg.bot_token, client_handler).expect("Error creating client");
    client.with_framework(
        StandardFramework::new()
            .configure(|c| c.prefix("!").allow_dm(false).case_insensitivity(true))
            .group(&TERRARIA_GROUP),
    );

    {
        // Add server stdin named pipe to client's shared data
        let mut data = client.data.write();
        data.insert::<DbClient>(db_pool_serenity);
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
        db_client_logger,
    )
    .expect("Unable to start log file thread");

    terraria_pcap::parse_packets(
        client.cache_and_http.http.clone(),
        ChannelId(cfg.bridge_channel_id),
        &cfg.tcpdump.interface,
        cfg.tcpdump.port,
        db_client_pcap,
    )
    .expect("Unable to start packet parsing thread");

    if let Err(e) = client.start() {
        eprintln!("An error occurred while running the client: {:?}", e);
    }
}

#[command]
fn deaths(ctx: &mut Context, msg: &Message) -> CommandResult {
    let data = ctx.data.read();
    let pool = data
        .get::<DbClient>()
        .expect("Failed to get database pool from context");
    let mut db = pool.get().expect("Failed to get connection from pool");

    match db.query("SELECT victim FROM death", &[]) {
        Err(e) => Err(CommandError(format!("Unable to query deaths: {}", e))),
        Ok(rows) => {
            let mut death_map: HashMap<String, u32> = HashMap::new();
            for row in rows {
                let victim: String = row.get(0);
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
                content.push_str(&format!("{} - {}\n", death.0, death.1));
            }

            if let Err(e) = msg.channel_id.say(ctx.http(), content) {
                return Err(CommandError(format!(
                    "Error replying to deaths command: {}",
                    e
                )));
            }

            Ok(())
        }
    }
}

#[command]
fn playing(_ctx: &mut Context, _msg: &Message) -> CommandResult {
    if let Err(e) = Command::new("tmux")
        .args(&["send-keys", "-t", "terraria", "playing\r\n"])
        .output()
    {
        return Err(CommandError(format!("Error writing to tmux pane: {}", e)));
    }

    Ok(())
}

// "tail"s server logfile, sending new lines to discord
fn send_loglines(
    filename: &str,
    http: Arc<Http>,
    channel_id: ChannelId,
    mut db: postgres::Client,
) -> Result<(), Box<dyn Error>> {
    let tail = Command::new("tail")
        .stdout(Stdio::piped())
        .args(&["-n", "0", "-F", filename])
        .spawn()?;

    let insert_join = db.prepare_typed(
        "INSERT INTO server_join(username) VALUES ($1)",
        &[Type::VARCHAR],
    )?;
    let insert_leave = db.prepare_typed(
        "INSERT INTO server_leave(username) VALUES ($1)",
        &[Type::VARCHAR],
    )?;
    let insert_message = db.prepare_typed(
        "INSERT INTO  message(author, content) VALUES ($1, $2)",
        &[Type::VARCHAR, Type::VARCHAR],
    )?;

    // Look for chat messages, joins, and leaves
    let chat_regex = Regex::new("^(?:: )*<(?P<user>.+?)> (?P<message>.+)$").unwrap();
    let join_leave_regex =
        Regex::new("^(?:: )*(?P<user>\\S.*) has (?P<status>joined|left)\\.$").unwrap();
    let playing_regex =
        Regex::new("^(?:: )*(?P<user>.+?) \\((?:\\d{1,3}\\.){3}\\d{1,3}:\\d+\\)$").unwrap();
    let connected_regex = Regex::new("^(?:: )*(\\w+ players? connected\\.)$").unwrap();

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
            if let Some(caps) = chat_regex.captures(line) {
                let user = &caps["user"];
                if user != "Server" {
                    let message = &caps["message"];
                    if let Err(e) =
                        channel_id.say(&http, format!("<{}> {}", &caps["user"], &caps["message"]))
                    {
                        eprintln!("Unable to send chat to discord: {}", e);
                    }
                    if let Err(e) = db.execute(&insert_message, &[&user, &message]) {
                        eprintln!("Unable to insert terraria message into db: {}", e);
                    }
                }
            } else if let Some(caps) = join_leave_regex.captures(line) {
                let user = &caps["user"];
                let status = &caps["status"];
                if let Err(e) =
                    channel_id.say(&http, format!("{} has {}", &caps["user"], &caps["status"]))
                {
                    eprintln!("Unable to send chat to discord: {}", e);
                }
                let statement = match status {
                    "joined" => &insert_join,
                    "left" => &insert_leave,
                    _ => continue,
                };
                if let Err(e) = db.execute(statement, &[&user]) {
                    eprintln!("Error inserting terraria user status: {}", e);
                }
            } else if let Some(caps) = playing_regex.captures(line) {
                if let Err(e) = channel_id.say(&http, &caps["user"]) {
                    eprintln!("Unable to send playing to discord: {}", e);
                }
            } else if let Some(caps) = connected_regex.captures(line) {
                if let Err(e) = channel_id.say(&http, &caps[1]) {
                    eprintln!("Unable to send playing count to discord: {}", e);
                }
            }
        }
    });

    Ok(())
}
