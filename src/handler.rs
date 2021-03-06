use serenity::model::channel::Message;
use serenity::model::gateway::{Activity, Ready};
use serenity::model::id::UserId;
use serenity::prelude::*;
use std::process::Command;

struct OwnUserId;

impl TypeMapKey for OwnUserId {
    type Value = UserId;
}

pub struct Handler {
    pub playing: Option<String>,
    pub bridge_channel_id: u64,
}

impl EventHandler for Handler {
    fn message(&self, ctx: Context, msg: Message) {
        // Ignore any messages not in the bridge channel
        if msg.channel_id.as_u64() != &self.bridge_channel_id || msg.content.find('!') == Some(0) {
            return;
        }

        // Try to get our ID from shared data
        let mut own_id = {
            let data = ctx.data.read();
            if let Some(id) = data.get::<OwnUserId>() {
                Some(*id)
            } else {
                None
            }
        };

        if own_id.is_none() {
            // If we couldn't get our ID from shared data (failed in ready?) then try to get it from http/cache
            match ctx.http.get_current_user() {
                Err(e) => eprintln!("Error getting self: {}", e),
                Ok(me) => {
                    // Set ID in shared data if we got it this time
                    let mut data = ctx.data.write();
                    data.insert::<OwnUserId>(me.id);
                    own_id = Some(me.id);
                }
            }
        }

        let mut content = msg
            .content
            .trim()
            .replace(|c: char| !c.is_ascii(), "")
            .replace('\n', " ")
            .replace(|c: char| c.is_ascii_control(), "");
        content.truncate(100);

        if !content.is_empty() {
            if let Some(own_id) = own_id {
                // If we know who we are and we didn't send this message
                if msg.author.id != own_id {
                    let author_name = match msg.author_nick(ctx.http) {
                        Some(nick) => nick,
                        None => msg.author.name,
                    };
                    if let Err(e) = Command::new("tmux")
                        .args(&[
                            "send-keys",
                            "-t",
                            "terraria",
                            &format!("say {}: {}\n", author_name, content),
                        ])
                        .output()
                    {
                        eprintln!("Error writing message to tmux pane: {}", e);
                    }
                }
            }
        }
    }

    fn ready(&self, ctx: Context, _: Ready) {
        if let Some(playing) = &self.playing {
            ctx.set_activity(Activity::playing(playing));
        }

        // Get our user ID and save to shared data
        match ctx.http.get_current_user() {
            Err(e) => eprintln!("Error getting self: {}", e),
            Ok(me) => {
                let mut data = ctx.data.write();
                data.insert::<OwnUserId>(me.id);
            }
        }
    }
}
