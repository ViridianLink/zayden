use std::fmt::Display;

use serenity::all::CreateUnfurledMediaItem;

#[derive(Clone, Copy)]
pub enum Weapon {
    LordOfWolves,
    Queenbreaker,
}

impl Display for Weapon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LordOfWolves => write!(f, "<:lord_of_wolves:1395198273932890243>"),
            Self::Queenbreaker => write!(f, "<:queenbreaker:1395198262264463410>"),
        }
    }
}

impl From<Weapon> for CreateUnfurledMediaItem<'static> {
    fn from(_value: Weapon) -> Self {
        // TODO: use per-weapon Bungie CDN URLs once confirmed
        Self::new(
            "https://www.bungie.net/common/destiny2_content/icons/6bd65ae8981e4cac3c00825abedd3fbb.jpg",
        )
    }
}
