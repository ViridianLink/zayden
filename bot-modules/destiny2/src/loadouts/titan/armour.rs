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
};

#[derive(Clone, Copy)]
pub enum Helmet {
    Any([HelmetMod; 3]),
}

impl ArmourItem for Helmet {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods) => mods.map(|m| Box::new(m) as Box<dyn Display>),
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/0f581262927001f7db9d95a40e9b2189.jpg"
            },
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Helmet {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Titan Helmet",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Gauntlets {
    Any([ArmsMod; 3]),
    AshenWake([ArmsMod; 3]),
}

impl ArmourItem for Gauntlets {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods) | Self::AshenWake(mods) => {
                mods.map(|m| Box::new(m) as Box<dyn Display>)
            },
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
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Gauntlets {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Titan Gauntlets",
            Self::AshenWake(_) => "Ashen Wake",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Plate {
    Any([ChestMod; 3]),
    HeartOfInmostLight([ChestMod; 3]),
}

impl ArmourItem for Plate {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods) => mods.map(|m| Box::new(m) as Box<dyn Display>),
            Self::HeartOfInmostLight(mods) => {
                mods.map(|m| Box::new(m) as Box<dyn Display>)
            },
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
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Plate {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Titan Chest Armour",
            Self::HeartOfInmostLight(_) => "Heart of Inmost Light",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Greaves {
    Any([LegsMod; 3]),
    PeregrineGreaves([LegsMod; 3]),
}

impl ArmourItem for Greaves {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods) | Self::PeregrineGreaves(mods) => {
                mods.map(|m| Box::new(m) as Box<dyn Display>)
            },
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
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Greaves {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Titan Leg Armour",
            Self::PeregrineGreaves(_) => "Peregrine Greaves",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Mark {
    Any([ClassItemMod; 3]),
}

impl ArmourItem for Mark {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods) => mods.map(|m| Box::new(m) as Box<dyn Display>),
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/4a80cc7898c2e510cb3e6c4a42982c42.jpg"
            },
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Mark {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Titan Mark",
        };

        write!(f, "{s}")
    }
}
