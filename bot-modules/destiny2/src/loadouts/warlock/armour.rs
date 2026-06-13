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
pub enum Hood {
    Any([HelmetMod; 3]),
    SkullOfDireAhamkara([HelmetMod; 3]),
    Deimosuffusion([HelmetMod; 3]),
}

impl ArmourItem for Hood {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods)
            | Self::SkullOfDireAhamkara(mods)
            | Self::Deimosuffusion(mods) => mods.map(box_display),
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/1cb2285f74ece98b03e170a3f8d9abdc.jpg"
            },
            Self::SkullOfDireAhamkara(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/19137504db8dcd63bb852f5324bbbbb3.jpg"
            },
            Self::Deimosuffusion(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/8d4cf66f37462c79069095736a4d7bb0.jpg"
            },
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Hood {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Warlock Helmet",
            Self::SkullOfDireAhamkara(_) => "Skull of Dire Ahamkara",
            Self::Deimosuffusion(_) => "Deimosuffusion",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Gloves {
    Any([ArmsMod; 3]),
}

impl ArmourItem for Gloves {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods) => mods.map(box_display),
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/bfece8a540293e1ac584d894caaa7258.jpg"
            },
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Gloves {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Warlock Gauntlets",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Robes {
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
            Self::Any(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/9fc0d6f0828aea5abe2f13354c6e63b5.jpg"
            },
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

#[derive(Clone, Copy)]
pub enum Boots {
    Any([LegsMod; 3]),
    BootsOfTheAssembler([LegsMod; 3]),
}

impl ArmourItem for Boots {
    fn mods(&self) -> [Box<dyn Display>; 3] {
        match self {
            Self::Any(mods) | Self::BootsOfTheAssembler(mods) => {
                mods.map(box_display)
            },
        }
    }

    fn as_unfurled_media_item<'a>(&self) -> CreateUnfurledMediaItem<'a> {
        let url = match self {
            Self::Any(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/1c3ae268b2f129c252f0609fe52b8028.jpg"
            },
            Self::BootsOfTheAssembler(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/4f54f0e8ad3cf58ff4525347c31c652b.jpg"
            },
        };

        CreateUnfurledMediaItem::new(url)
    }
}

impl Display for Boots {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Any(_) => "Any Warlock Boots",
            Self::BootsOfTheAssembler(_) => "Boots of the Assembler",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Bond {
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
            Self::Any(_) => {
                "https://www.bungie.net/common/destiny2_content/icons/9b5a20e8b090754429762e5836f4131f.jpg"
            },
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
