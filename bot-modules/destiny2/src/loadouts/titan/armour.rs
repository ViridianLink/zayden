use std::fmt;
use std::fmt::{Display, Formatter};

use serenity::all::CreateUnfurledMediaItem;

use super::super::{
    ArmourItem,
    ArmsMod,
    ChestMod,
    ClassItemMod,
    HelmetMod,
    LegsMod,
    box_display,
};

#[derive(Clone, Copy)]
pub enum Helmet {
    Any([HelmetMod; 3]),
    WillbreakersWatch([HelmetMod; 3]),
}

impl ArmourItem for Helmet {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods) | Self::WillbreakersWatch(mods) => mods.map(box_display),
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/0f581262927001f7db9d95a40e9b2189.jpg"
            },
            Self::WillbreakersWatch(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/8483dd383731573ac8921490f1721f75.jpg"
            },
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Helmet {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Titan Helmet",
            Self::WillbreakersWatch(_) => "Willbreaker's Watch",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Arms {
    Any([ArmsMod; 3]),
    AshenWake([ArmsMod; 3]),
    WillbreakersFists([ArmsMod; 3]),
}

impl ArmourItem for Arms {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods)
            | Self::AshenWake(mods)
            | Self::WillbreakersFists(mods) => mods.map(box_display),
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/f378e5316a99404cab087ecdce699758.jpg"
            },
            Self::AshenWake(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/9d29d23f2920b16378127d9603370722.jpg"
            },
            Self::WillbreakersFists(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/4586c65813639d37c11e665ae45cfc98.jpg"
            },
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Arms {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Titan Gauntlets",
            Self::AshenWake(_) => "Ashen Wake",
            Self::WillbreakersFists(_) => "Willbreaker's Fists",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Chest {
    Any([ChestMod; 3]),
    HeartOfInmostLight([ChestMod; 3]),
    HallowfireHeart([ChestMod; 3]),
}

impl ArmourItem for Chest {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods)
            | Self::HeartOfInmostLight(mods)
            | Self::HallowfireHeart(mods) => mods.map(box_display),
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/8cc0f113cf22b387d05b6b040250dc64.jpg"
            },
            Self::HeartOfInmostLight(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/34f23604746fc260a2153e93ccfaec7f.jpg"
            },
            Self::HallowfireHeart(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/8aacd27b76e68afe6287a3984adeb601.jpg"
            },
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Chest {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Titan Chest Armour",
            Self::HeartOfInmostLight(_) => "Heart of Inmost Light",
            Self::HallowfireHeart(_) => "Hallowfire Heart",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Legs {
    Any([LegsMod; 3]),
    PeregrineGreaves([LegsMod; 3]),
    SmokeJumperBoots([LegsMod; 3]),
}

impl ArmourItem for Legs {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods)
            | Self::PeregrineGreaves(mods)
            | Self::SmokeJumperBoots(mods) => mods.map(box_display),
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/e7464694d858cad360801638ccd96b07.jpg"
            },
            Self::PeregrineGreaves(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/b5a4ec9e6e4a0ec3f83ef47f406a8fa6.jpg"
            },
            Self::SmokeJumperBoots(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/15c3a83be21ce72bc8622ad059527ab7.jpg"
            },
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Legs {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Titan Leg Armour",
            Self::PeregrineGreaves(_) => "Peregrine Greaves",
            Self::SmokeJumperBoots(_) => "Smoke Jumper Boots",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Mark {
    Any([ClassItemMod; 3]),
    SmokeJumperMark([ClassItemMod; 3]),
}

impl ArmourItem for Mark {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods) | Self::SmokeJumperMark(mods) => mods.map(box_display),
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/4a80cc7898c2e510cb3e6c4a42982c42.jpg"
            },
            Self::SmokeJumperMark(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/eb7f47e45fac99fd61abcd6bc2be938f.jpg"
            },
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Mark {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Titan Mark",
            Self::SmokeJumperMark(_) => "Smoke Jumper Mark",
        };

        write!(f, "{s}")
    }
}
