use std::collections::HashMap;

pub fn get() -> HashMap<&'static str, &'static str> {
    [
        ("HasBeenDefeated_Single", "{0} has been defeated!"),
        ("HasBeenDefeated_Plural", "{0} have been defeated!"),
        ("HasAwoken", "{0} has awoken!"),
        ("HasArrived", "{0} has arrived!"),
    ]
    .iter()
    .copied()
    .collect()
}
