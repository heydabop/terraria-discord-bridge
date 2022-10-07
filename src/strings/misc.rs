use std::collections::HashMap;

#[allow(clippy::too_many_lines)]
pub fn get() -> HashMap<&'static str, &'static str> {
    [
        ("ForceWaterSettling", "Forcing water to settle."),
        ("WaterSettled", "Water has been settled."),
        ("ResolutionChanged", "Resolution changed to: {0}x{1}."),
        ("ShortDays", "d"),
        ("ShortHours", "h"),
        ("ShortMinutes", "m"),
        ("ShortSeconds", "s"),
        (
            "CombatBookUsed",
            "The book's knowledge empowers your villagers!",
        ),
        ("Fruit", "Fruit"),
        ("CanBePlacedInVanity", "Can be worn in vanity slots"),
        (
            "StartedVictoryXmas",
            "The spirit of Christmas spreads cheer...",
        ),
        ("EndedVictoryXmas", "The spirit of Christmas fades..."),
        (
            "StartedVictoryHalloween",
            "The spirit of Halloween penetrates the air...",
        ),
        ("EndedVictoryHalloween", "The spirit of Halloween rests..."),
        (
            "LicenseCatUsed",
            "The license teleports away to the cat delivery service...",
        ),
        (
            "LicenseDogUsed",
            "The license teleports away to the dog delivery service...",
        ),
        (
            "LicenseBunnyUsed",
            "The license teleports away to the bunny delivery service...",
        ),
        (
            "LicenseSlimeUsed",
            "The license teleports away to the slime delivery service...",
        ),
        ("Ebonstone", "Ebonstone"),
        ("Crimstone", "Crimstone"),
        ("Balloon", "Balloon"),
        (
            "PumpkinMoonScore",
            "The Pumpkin Moon has passed! (Score: {0})",
        ),
        ("FrostMoonScore", "The Frost Moon has passed! (Score: {0})"),
        (
            "PetExchangeFail",
            "Wait for your pet to move in before exchanging it!",
        ),
        ("PetExchangeSuccess", "Pet Exchange: Successful!"),
        ("Cockatiel", "Cockatiel"),
        ("Macaw", "Macaw"),
        ("CloudBalloon", "Cloud Ballooon"),
        ("BlizzardBalloon", "Blizzard Balloon"),
        ("SandstormBalloon", "Sandstorm Balloon"),
        ("CombatBookVolumeTwoUsed", "{$Misc.CombatBookUsed}"),
        (
            "PeddlersSatchelUsed",
            "The Traveling Merchant's satchel deepens!",
        ),
    ]
    .iter()
    .copied()
    .collect()
}
