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
    GravitonForfeit([HelmetMod; 3]),
}

impl ArmourItem for Helmet {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods) | Self::GravitonForfeit(mods) => mods.map(box_display),
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/d2abc2257f85934b8ff763e563f02cd9.jpg"
            },
            Self::GravitonForfeit(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/8571524c477cee2fc3e2dedbaf7aa8bb.jpg"
            },
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Helmet {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Hunter Helmet",
            Self::GravitonForfeit(_) => "Graviton Forfeit",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Gauntlets {
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
            Self::Any(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/1cfe58452f5dae674b7f6d0f816e9592.jpg"
            },
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

#[derive(Clone, Copy)]
pub enum Vest {
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
            Self::Any(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/4809aed7a4b4803dd9235baa9a36ee36.jpg"
            },
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

#[derive(Clone, Copy)]
pub enum Legs {
    Any([LegsMod; 3]),
    FortunesFavor([LegsMod; 3]),
}

impl ArmourItem for Legs {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods) | Self::FortunesFavor(mods) => mods.map(box_display),
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/1cfe58452f5dae674b7f6d0f816e9592.jpg"
            },
            Self::FortunesFavor(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/aef3f0c6823ea0564d94ff81b40552f0.jpg"
            },
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Legs {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Hunter Leg Armour",
            Self::FortunesFavor(_) => "Fortune's Favor",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Cloak {
    Any([ClassItemMod; 3]),
    Relativism([RelativismTrait; 2], [ClassItemMod; 3]),
}

impl ArmourItem for Cloak {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods) | Self::Relativism(_, mods) => mods.map(box_display),
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/363fd4e1311408d0f5400f6d9579cf2f.jpg"
            },
            Self::Relativism(..) => {
                "https://www.bungie.net/common/destiny2_content/icons/58ae63b773059e5bd7a4d43298f73a50.jpg"
            },
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Cloak {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Hunter Cloak",
            Self::Relativism(traits, _) => {
                return write!(f, "Relativism ({} + {})", traits[0], traits[1]);
            },
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum RelativismTrait {
    Assassin,
    InmostLight,
    Ophidian,
    Dragon,
    Galanor,
    Foetracer,
    Caliban,
    Renewal,
    Apotheosis,
    StarEater,
    Synthoceps,
    Verity,
    Cyrtarachne,
    Gyrfalcon,
    Liar,
    Wormhusk,
    Coyote,
    Claw,
}

impl Display for RelativismTrait {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Assassin => "Assassin",
            Self::InmostLight => "Inmost Light",
            Self::Ophidian => "Ophidian",
            Self::Dragon => "Dragon",
            Self::Galanor => "Galanor",
            Self::Foetracer => "Foetracer",
            Self::Caliban => "Caliban",
            Self::Renewal => "Renewal",
            Self::Apotheosis => "Apotheosis",
            Self::StarEater => "Star-Eater",
            Self::Synthoceps => "Synthoceps",
            Self::Verity => "Verity",
            Self::Cyrtarachne => "Cyrtarachne",
            Self::Gyrfalcon => "Gyrfalcon",
            Self::Liar => "Liar",
            Self::Wormhusk => "Wormhusk",
            Self::Coyote => "Coyote",
            Self::Claw => "Claw",
        };

        write!(f, "{s}")
    }
}
