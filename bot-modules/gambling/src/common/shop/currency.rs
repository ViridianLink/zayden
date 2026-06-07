use std::fmt::Debug;
use std::str::FromStr;

use zayden_core::EmojiCache;

use crate::error::Result;
use crate::{GEM, GamblingError};

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
    #[must_use]
    pub const fn craft_req(&self, _: &EmojiCache) -> [Option<(Self, u16)>; 4] {
        match self {
            Self::Tech => {
                [Some((Self::Coal, 10)), Some((Self::Iron, 5)), None, None]
            },
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
            Self::Coins
            | Self::Gems
            | Self::Coal
            | Self::Iron
            | Self::Gold
            | Self::Redstone
            | Self::Lapis
            | Self::Diamonds
            | Self::Emeralds => [None, None, None, None],
        }
    }

    pub fn emoji(&self, emojis: &EmojiCache) -> Result<String> {
        let s = match self {
            Self::Coins => format!(
                "<:coin:{}>",
                emojis.get("heads").ok_or_else(|| GamblingError::Internal(
                    "emoji 'heads' not in cache".to_string()
                ))?
            ),
            Self::Gems => return Ok(GEM.to_string()),
            Self::Tech => format!(
                "<:tech:{}>",
                emojis.get("tech").ok_or_else(|| GamblingError::Internal(
                    "emoji 'tech' not in cache".to_string()
                ))?
            ),
            Self::Utility => format!(
                "<:utility:{}>",
                emojis.get("utility").ok_or_else(|| GamblingError::Internal(
                    "emoji 'utility' not in cache".to_string()
                ))?
            ),
            Self::Production => format!(
                "<:production:{}>",
                emojis.get("tech").ok_or_else(|| GamblingError::Internal(
                    "emoji 'tech' not in cache".to_string()
                ))?
            ),
            Self::Coal => format!(
                "<:coal:{}>",
                emojis.get("coal").ok_or_else(|| GamblingError::Internal(
                    "emoji 'coal' not in cache".to_string()
                ))?
            ),
            Self::Iron => format!(
                "<:iron:{}>",
                emojis.get("iron").ok_or_else(|| GamblingError::Internal(
                    "emoji 'iron' not in cache".to_string()
                ))?
            ),
            Self::Gold => format!(
                "<:gold:{}>",
                emojis.get("gold").ok_or_else(|| GamblingError::Internal(
                    "emoji 'gold' not in cache".to_string()
                ))?
            ),
            Self::Redstone => format!(
                "<:redstone:{}>",
                emojis.get("redstone").ok_or_else(|| GamblingError::Internal(
                    "emoji 'redstone' not in cache".to_string()
                ))?
            ),
            Self::Lapis => format!(
                "<:lapis:{}>",
                emojis.get("lapis").ok_or_else(|| GamblingError::Internal(
                    "emoji 'lapis' not in cache".to_string()
                ))?
            ),
            Self::Diamonds => format!(
                "<:diamond:{}>",
                emojis.get("diamond").ok_or_else(|| GamblingError::Internal(
                    "emoji 'diamond' not in cache".to_string()
                ))?
            ),
            Self::Emeralds => format!(
                "<:emerald:{}>",
                emojis.get("emerald").ok_or_else(|| GamblingError::Internal(
                    "emoji 'emerald' not in cache".to_string()
                ))?
            ),
        };

        Ok(s)
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
            s => {
                tracing::warn!(
                    currency = s,
                    "ShopCurrency::from_str: unknown currency"
                );

                Err(())
            },
        }
    }
}
