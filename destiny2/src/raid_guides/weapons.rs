use std::fmt::Display;

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

// impl<'a> From<Weapon> for CreateUnfurledMediaItem<'a> {
//     fn from(value: Weapon) -> Self {
//         match value {
//             Weapon::LordOfWolves => CreateUnfurledMediaItem::new(
//                 "https://www.bungie.net/common/destiny2_content/icons/6bd65ae8981e4cac3c00825abedd3fbb.jpg",
//             ),
//             Weapon::Queenbreaker => CreateUnfurledMediaItem::new(
//                 "https://www.bungie.net/common/destiny2_content/icons/6bd65ae8981e4cac3c00825abedd3fbb.jpg",
//             ),
//         }
//     }
// }
