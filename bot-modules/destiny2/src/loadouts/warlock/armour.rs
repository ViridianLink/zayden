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

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum Helmet {
    Any([HelmetMod; 3]),
}

impl ArmourItem for Helmet {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods) => mods.map(box_display),
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => "",
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Helmet {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Warlock Helmet",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum Gauntlets {
    Any([ArmsMod; 3]),
}

impl ArmourItem for Gauntlets {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods) => mods.map(box_display),
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => "",
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Gauntlets {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Warlock Gauntlets",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum Robes {
    Any([ChestMod; 3]),
}

impl ArmourItem for Robes {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods) => mods.map(box_display),
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => "",
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Robes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Warlock Robes",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum Boots {
    Any([LegsMod; 3]),
}

impl ArmourItem for Boots {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods) => mods.map(box_display),
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => "",
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Boots {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Warlock Boots",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum Bond {
    Any([ClassItemMod; 3]),
}

impl ArmourItem for Bond {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods) => mods.map(box_display),
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => "",
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Bond {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Warlock Bond",
        };

        write!(f, "{s}")
    }
}
