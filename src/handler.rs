use serenity::model::gateway::{Activity, Ready};
use serenity::model::id::UserId;
use serenity::prelude::*;

struct OwnUserId;

impl TypeMapKey for OwnUserId {
    type Value = UserId;
}

pub struct Handler {
    pub playing: Option<String>,
    pub bridge_channel_id: u64,
}

impl EventHandler for Handler {
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
