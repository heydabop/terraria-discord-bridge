use std::collections::HashMap;

pub fn get() -> HashMap<&'static str, &'static str> {
    [
        ("Slain", "{0} was slain"),
        ("Eviscerated", "{0} was eviscerated"),
        ("Murdered", "{0} was murdered"),
        ("FaceTornOff", "{0}'s face was torn off"),
        ("EntrailsRippedOut", "{0}'s entrails were ripped out"),
        ("Destroyed", "{0} was destroyed"),
        ("SkullCrushed", "{0}'s skull was crushed"),
        ("Massacred", "{0} got massacred"),
        ("Impaled", "{0} got impaled"),
        ("TornInHalf", "{0} was torn in half"),
        ("Decapitated", "{0} was decapitated"),
        ("ArmTornOff", "{0} let their arms get torn off"),
        (
            "InnardsBecameOutards",
            "{0} watched their innards become outards",
        ),
        ("Dissected", "{0} was brutally dissected"),
        ("ExtremitiesDetached", "{0}'s extremities were detached"),
        ("Mangled", "{0}'s body was mangled"),
        ("Ruptured", "{0}'s vital organs were ruptured"),
        ("PileOfFlesh", "{0} was turned into a pile of flesh"),
        ("Removed", "{0} was removed from {1}"),
        ("Snapped", "{0} got snapped in half"),
        ("Cut", "{0} was cut down the middle"),
        ("Chopped", "{0} was chopped up"),
        ("Plead", "{0}'s plead for death was answered"),
        ("Ripped", "{0}'s meat was ripped off the bone"),
        ("Flailing", "{0}'s flailing about was finally stopped"),
        ("HeadRemoved", "{0} had their head removed"),
    ]
    .iter()
    .cloned()
    .collect()
}
