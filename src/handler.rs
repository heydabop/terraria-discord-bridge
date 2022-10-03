use serenity::model::gateway::{Activity, Ready};
use serenity::prelude::*;

pub struct Handler {
    pub playing: Option<String>,
    pub bridge_channel_id: u64,
}

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: Ready) {
        if let Some(playing) = &self.playing {
            ctx.set_activity(Activity::playing(playing)).await;
        }
    }
}
