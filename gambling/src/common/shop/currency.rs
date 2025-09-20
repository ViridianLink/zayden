use std::fmt::Debug;
use std::str::FromStr;

use zayden_core::EmojiCache;

use crate::GEM;

#[derive(Clone, Copy)]
pub enum ShopCurrency {
    Coins,
    Gems,
    Tech,
    Utility,
    Production,
    Coal,
    Iron,
    Gold,
    Redstone,
    Lapis,
    Diamonds,
    Emeralds,
}

impl ShopCurrency {
    pub fn craft_req(&self, emojis: &EmojiCache) -> [Option<(Self, u16)>; 4] {
        match self {
            Self::Tech => [Some((Self::Coal, 10)), Some((Self::Iron, 5)), None, None],
            Self::Utility => [
                Some((Self::Coal, 15)),
                Some((Self::Gold, 10)),
                Some((Self::Diamonds, 5)),
                Some((Self::Emeralds, 1)),
            ],
            Self::Production => [
                Some((Self::Gold, 100)),
                Some((Self::Lapis, 500)),
                Some((Self::Redstone, 125)),
                None,
            ],
            c => unreachable!("Invalid currency {}", c.emoji(emojis)),
        }
    }

    pub fn emoji(&self, emojis: &EmojiCache) -> String {
        match self {
            ShopCurrency::Coins => format!("<:coin:{}>", emojis.get("heads").unwrap()),
            ShopCurrency::Gems => GEM.to_string(),
            ShopCurrency::Tech => format!("<:tech:{}>", emojis.get("tech").unwrap()),
            ShopCurrency::Utility => format!("<:utility:{}>", emojis.get("utility").unwrap()),
            ShopCurrency::Production => format!("<:production:{}>", emojis.get("tech").unwrap()),
            ShopCurrency::Coal => format!("<:coal:{}>", emojis.get("coal").unwrap()),
            ShopCurrency::Iron => format!("<:iron:{}>", emojis.get("iron").unwrap()),
            ShopCurrency::Gold => format!("<:gold:{}>", emojis.get("gold").unwrap()),
            ShopCurrency::Redstone => format!("<:redstone:{}>", emojis.get("redstone").unwrap()),
            ShopCurrency::Lapis => format!("<:lapis:{}>", emojis.get("lapis").unwrap()),
            ShopCurrency::Diamonds => format!("<:diamond:{}>", emojis.get("diamond").unwrap()),
            ShopCurrency::Emeralds => format!("<:emerald:{}>", emojis.get("emerald").unwrap()),
        }
    }
}

impl Debug for ShopCurrency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Coins => write!(f, "Coins"),
            Self::Gems => write!(f, "Gems"),
            Self::Tech => write!(f, "Tech Pack"),
            Self::Utility => write!(f, "Utility Pack"),
            Self::Production => write!(f, "Production Pack"),
            Self::Coal => write!(f, "Coal"),
            Self::Iron => write!(f, "Iron"),
            Self::Gold => write!(f, "Gold"),
            Self::Redstone => write!(f, "Redstone"),
            Self::Lapis => write!(f, "Lapis"),
            Self::Diamonds => write!(f, "Diamonds"),
            Self::Emeralds => write!(f, "Emeralds"),
        }
    }
}

impl FromStr for ShopCurrency {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "tech" => Ok(Self::Tech),
            "utility" => Ok(Self::Utility),
            "production" => Ok(Self::Production),
            s => unimplemented!("Currency {s} has not been implemented"),
        }
    }
}
