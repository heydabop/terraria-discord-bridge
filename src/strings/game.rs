use std::collections::HashMap;

pub fn get() -> HashMap<&'static str, &'static str> {
    [
        ("Wave", "Wave: {0}"),
        ("FirstWave", "First Wave"),
        ("FinalWave", "Final Wave"),
        ("InvasionWave_Type1", "{0}: {1}"),
        ("InvasionWave_Type2", "{0}: {1}, and {2}"),
        ("InvasionWave_Type3", "{0}: {1}, {2}, and {3}"),
        ("InvasionWave_Type4", "{0}: {1}, {2}, {3}, and {4}"),
        ("InvasionWave_Type5", "{0}: {1}, {2}, {3}, {4}, and {5}"),
        (
            "InvasionWave_Type6",
            "{0}: {1}, {2}, {3}, {4}, {5}, and {6}",
        ),
        (
            "InvasionWave_Type7",
            "{0}: {1}, {2}, {3}, {4}, {5}, {6}, and {7}",
        ),
        (
            "InvasionWave_Type8",
            "{0}: {1}, {2}, {3}, {4}, {5}, {6}, {7}, and {8}",
        ),
        (
            "BallBounceResult",
            "{0} was hit {1} times before touching the ground!",
        ),
        ("JoinGreeting", "Current players: {0}."),
        ("BedObstructed", "Your bed is obstructed."),
        ("PvPFlag", "(PvP)"),
        ("DroppedCoins", "dropped {0}"),
        ("InvasionPoints", "{0} points"),
        ("WaveMessage", "Wave {0}: {1}"),
        ("WaveCleared", "Cleared {0}"),
        ("TeleportTo", "Teleport to {0}"),
        ("HasTeleportedTo", "{0} has teleported to {1}"),
        ("Time", "Time: {0}"),
        ("NPCTitle", "{0} the {1}"),
        ("PlayerDeathTime", "{0} died {1} ago"),
        ("SpawnPointRemoved", "Spawn point removed!"),
        ("SpawnPointSet", "Spawn point set!"),
        ("RedWires", "Red Wires"),
        ("BlueWires", "Blue Wires"),
        ("GreenWires", "Green Wires"),
        ("YellowWires", "Yellow Wires"),
        ("Actuators", "Actuators"),
        (
            "EnemiesDefeatedAnnouncement",
            "The {0}th {1} has been defeated!",
        ),
        (
            "EnemiesDefeatedByAnnouncement",
            "{0} has defeated the {1}th {2}!",
        ),
        ("HouseMissing_1", "This house is missing {0}."),
        ("HouseMissing_2", "This house is missing {0} and {1}."),
        ("HouseMissing_3", "This house is missing {0}, {1}, and {2}."),
        (
            "HouseMissing_4",
            "This house is missing {0}, {1}, {2}, and {3}.",
        ),
        ("HouseLightSource", "a light source"),
        ("HouseDoor", "a door"),
        ("HouseTable", "a table"),
        ("HouseChair", "a chair"),
        ("BirthdayParty_1", "Looks like {0} is throwing a party"),
        (
            "BirthdayParty_2",
            "Looks like {0} & {1} are throwing a party",
        ),
        (
            "BirthdayParty_3",
            "Looks like {0}, {1}, and {2} are throwing a party",
        ),
    ]
    .iter()
    .cloned()
    .collect()
}
