use std::collections::HashMap;

pub fn get() -> HashMap<&'static str, &'static str> {
    [
        ("HasBeenDefeated_Single", "{0} has been defeated!"),
        ("HasBeenDefeated_Plural", "{0} have been defeated!"),
        ("HasAwoken", "{0} has awoken!"),
        ("HasArrived", "{0} has arrived!"),
        (
            "HomelessArrived_0",
            "{0} has arrived, searching for a place to rest.",
        ),
        ("HomelessArrived_1", "{0} has arrived, looking for a home!"),
        ("HomelessArrived_2", "{0} has arrived, wishing for a house!"),
        ("HomelessArrived_3", "{0} has arrived, desiring shelter!"),
        (
            "HomelessArrived_4",
            "{0} has arrived, hoping to settle down!",
        ),
        (
            "HomelessArrived_5",
            "{0} has arrived, looking to settle nearby.",
        ),
    ]
    .iter()
    .copied()
    .collect()
}
