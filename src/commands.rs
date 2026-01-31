use crate::Data;
use std::collections::HashMap;
use std::fmt::Write as _;
use tokio::process::Command;
use tokio::time::{Duration, sleep};

type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;

/// Show players sorted by how many times they've died
#[poise::command(slash_command, prefix_command)]
pub async fn deaths(ctx: Context<'_>) -> Result<(), Error> {
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
pub async fn playing(ctx: Context<'_>) -> Result<(), Error> {
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
pub async fn version(ctx: Context<'_>) -> Result<(), Error> {
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
pub async fn update(
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
