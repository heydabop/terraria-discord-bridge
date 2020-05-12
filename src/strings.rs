mod death_source;
mod death_text;
mod death_text_generic;
mod item_name;
mod npc_name;
mod projectile_name;

use std::collections::HashMap;

pub fn get() -> HashMap<&'static str, HashMap<&'static str, &'static str>> {
    [
        ("DeathSource", death_source::get()),
        ("DeathText", death_text::get()),
        ("DeathTextGeneric", death_text_generic::get()),
        ("ItemName", item_name::get()),
        ("NPCName", npc_name::get()),
        ("ProjectileName", projectile_name::get()),
    ]
    .iter()
    .cloned()
    .collect()
}
