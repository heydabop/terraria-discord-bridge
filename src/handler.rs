use serenity::{
    model::{
        channel::Message,
        gateway::{Activity, Ready},
    },
    prelude::*,
};

pub struct Handler {
    pub playing: String,
    pub bridge_channel_id: u64,
}

impl EventHandler for Handler {
    fn message(&self, _: Context, msg: Message) {
        if msg.channel_id.as_u64() != &self.bridge_channel_id {
            return;
        }
        println!("{}", msg.content);
    }

    fn ready(&self, ctx: Context, _: Ready) {
        ctx.set_activity(Activity::playing(&self.playing));
    }
}
