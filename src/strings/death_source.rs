use std::collections::HashMap;

pub fn get() -> HashMap<&'static str, &'static str> {
    [
        ("Player", "{0} by {1}'s {2}."),
        ("NPC", "{0} by {1}."),
        ("Projectile", "{0} by {1}."),
    ]
    .iter()
    .copied()
    .collect()
}
