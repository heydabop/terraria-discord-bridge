use std::collections::HashMap;

#[allow(clippy::too_many_lines)]
pub fn get() -> HashMap<&'static str, &'static str> {
    [
        ("0", "A goblin army has been defeated!"),
        ("100", "Pick world evil"),
        ("101", "Corruption"),
        ("102", "Crimson"),
        ("103", "Random"),
        (
            "104",
            "Cannot be used without Etherian Mana until the Eternia Crystal has been defended",
        ),
        ("105", "Dragonfly"),
        ("106", "The horrors have arrived!"),
        ("107", "Mechdusa has awoken!"),
        ("108", "What a horrible night to have a curse."),
        ("10", "A horrible chill goes down your spine..."),
        ("11", "Screams echo around you..."),
        ("12", "Your world has been blessed with Cobalt!"),
        ("13", "Your world has been blessed with Mythril!"),
        ("14", "Your world has been blessed with Adamantite!"),
        (
            "15",
            "The ancient spirits of light and dark have been released.",
        ),
        ("19", "{0} was slain..."),
        ("1", "A goblin army is approaching from the west!"),
        ("20", "A solar eclipse is happening!"),
        ("21", "Your world has been blessed with Palladium!"),
        ("22", "Your world has been blessed with Orichalcum!"),
        ("23", "Your world has been blessed with Titanium!"),
        ("24", "The pirates have been defeated!"),
        ("25", "Pirates are approaching from the west!"),
        ("26", "Pirates are approaching from the east!"),
        ("27", "The pirates have arrived!"),
        ("28", "You feel vibrations from deep below..."),
        ("29", "This is going to be a terrible night..."),
        ("2", "A goblin army is approaching from the east!"),
        ("30", "The air is getting colder around you..."),
        ("31", "The Pumpkin Moon is rising..."),
        ("32", "The jungle grows restless..."),
        ("33", "Screams are echoing from the dungeon..."),
        ("34", "The Frost Moon is rising..."),
        ("35", "{0} has departed!"),
        ("36", "{0} has left!"),
        ("37", "Any"),
        ("38", "Pressure Plate"),
        ("39", " and increased life regeneration"),
        ("3", "A goblin army has arrived!"),
        ("40", "Increases life regeneration"),
        ("41", "Martians are invading!"),
        ("42", "The martians have been defeated!"),
        ("43", "Celestial creatures are invading!"),
        ("44", "Your mind goes numb..."),
        ("45", "You are overwhelmed with pain..."),
        ("46", "Otherworldly voices linger around you..."),
        ("47", "The Moon Lord has awoken!"),
        ("48", "The Twins have awoken!"),
        ("49", "You wake up from a strange dream..."),
        ("4", "The Frost Legion has been defeated!"),
        ("50", "have been defeated!"),
        ("51", "Lunar Fragment"),
        ("52", "Impending doom approaches..."),
        ("53", "Select"),
        ("54", "Take"),
        ("55", "Take One"),
        ("56", "Close"),
        ("57", "Grapple"),
        ("58", "Jump"),
        ("59", "Cycle hotbar"),
        ("5", "The Frost Legion is approaching from the west!"),
        ("60", "Attack"),
        ("61", "Build"),
        ("62", "Drink"),
        ("63", "Action"),
        ("64", "Switch menu"),
        ("65", "Place"),
        ("66", "Swap"),
        ("67", "Equip"),
        ("68", "Unequip"),
        ("69", "Show room flags"),
        ("6", "The Frost Legion is approaching from the east!"),
        ("70", "Check housing"),
        ("71", "Quick craft"),
        ("72", "Craft"),
        ("73", "Select"),
        ("74", "Trash"),
        ("75", "Sell"),
        ("76", "Transfer"),
        ("77", "Show visuals"),
        ("78", "Hide visuals"),
        ("79", "Use"),
        ("7", "The Frost Legion has arrived!"),
        ("80", "Talk"),
        ("81", "Read"),
        ("82", "Back"),
        ("83", "Favorite"),
        ("84", "You can't change teams inside your team's blocks!"),
        ("85", "Jungle Bug"),
        ("86", "Duck"),
        ("87", "Butterfly"),
        ("88", "Firefly"),
        ("89", "Wiring Options"),
        ("8", "The Blood Moon is rising..."),
        ("90", "Buy"),
        ("91", "Buy More"),
        ("92", "Sell"),
        ("93", "Craft more"),
        ("94", "Try Removing"),
        ("95", "Snail"),
        ("96", "Looks like "),
        ("97", " is throwing a party"),
        ("98", " are throwing a party"),
        ("99", "Party time's over!"),
        ("9", "You feel an evil presence watching you..."),
    ]
    .iter()
    .copied()
    .collect()
}
