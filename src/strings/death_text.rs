use std::collections::HashMap;

pub fn get() -> HashMap<&'static str, &'static str> {
    [
        ("Suffocated", "{0} couldn't breathe."),
        ("Poisoned", "{0} couldn't find the antidote."),
        ("Burned", "{0} couldn't put the fire out."),
        ("Electrocuted", "{0} couldn't contain the watts."),
        ("TriedToEscape", "{0} tried to escape."),
        ("WasLicked", "{0} was licked."),
        ("Teleport_1", "{0} didn't materialize"),
        (
            "Teleport_2_Male",
            "{0}'s legs appeared where his head should be.",
        ),
        (
            "Teleport_2_Female",
            "{0}'s legs appeared where her head should be.",
        ),
        ("Slain", "{0} was slain..."),
        ("Stabbed", "{0} was stabbed."),
        ("Default", "{0}."),
        ("Fell_1", "{0} fell to their death."),
        ("Fell_2", "{0} didn't bounce."),
        ("Drowned_1", "{0} forgot to breathe."),
        ("Drowned_2", "{0} is sleeping with the fish."),
        ("Drowned_3", "{0} drowned."),
        ("Drowned_4", "{0} is shark food."),
        ("Lava_1", "{0} got melted."),
        ("Lava_2", "{0} was incinerated."),
        ("Lava_3", "{0} tried to swim in lava."),
        ("Lava_4", "{0} likes to play in magma."),
        ("Petrified_1", "{0} shattered into pieces."),
        ("Petrified_2", "{0} can't be put back together again."),
        ("Petrified_3", "{0} needs to be swept up."),
        ("Petrified_4", "{0} just became another dirt pile."),
        ("Inferno", "{0} was consumed by the inferno."),
    ]
    .iter()
    .cloned()
    .collect()
}
