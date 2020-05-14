use std::collections::HashMap;

pub fn get() -> HashMap<&'static str, &'static str> {
    [("19", "{0} has joined."), ("20", "{0} has left.")]
        .iter()
        .cloned()
        .collect()
}
