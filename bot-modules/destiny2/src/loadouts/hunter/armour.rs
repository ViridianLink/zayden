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
            Self::Any(_) => "Any Hunter Helmet",
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
            Self::Any(_) => "Any Hunter Gauntlets",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum Vest {
    Any([ChestMod; 3]),
}

impl ArmourItem for Vest {
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

impl Display for Vest {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Hunter Chest Armour",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum Greaves {
    Any([LegsMod; 3]),
}

impl ArmourItem for Greaves {
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

impl Display for Greaves {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Hunter Leg Armour",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum Cloak {
    Any([ClassItemMod; 3]),
}

impl ArmourItem for Cloak {
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

impl Display for Cloak {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Hunter Cloak",
        };

        write!(f, "{s}")
    }
}
