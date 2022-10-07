mod announcement;
mod death_source;
mod death_text;
mod death_text_generic;
mod game;
mod item_name;
mod legacy_misc;
mod legacy_world_gen;
mod misc;
mod npc_name;
mod projectile_name;

use std::collections::HashMap;

pub fn get() -> HashMap<&'static str, HashMap<&'static str, &'static str>> {
    [
        ("Announcement", announcement::get()),
        ("DeathSource", death_source::get()),
        ("DeathText", death_text::get()),
        ("DeathTextGeneric", death_text_generic::get()),
        ("Game", game::get()),
        ("ItemName", item_name::get()),
        ("LegacyMisc", legacy_misc::get()),
        ("LegacyWorldGen", legacy_world_gen::get()),
        ("Misc", misc::get()),
        ("NPCName", npc_name::get()),
        ("ProjectileName", projectile_name::get()),
    ]
    .iter()
    .cloned()
    .collect()
}
