use serenity::{
    model::gateway::{Activity, Ready},
    prelude::*,
};

pub struct Handler {
    pub playing: String,
}

impl EventHandler for Handler {
    fn ready(&self, ctx: Context, _: Ready) {
        ctx.set_activity(Activity::playing(&self.playing));
    }
}
