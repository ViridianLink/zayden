use std::fmt;
use std::fmt::{Display, Formatter};

use super::super::{
    Abilities as AbilitiesTrait,
    Aspect as AspectTrait,
    PrismaticFragment,
    box_display,
};
use super::{ClassAbility, Jump};

#[derive(Clone, Copy)]
pub(crate) struct Abilities {
    pub super_: Super,
    pub class: ClassAbility,
    pub jump: Jump,
    pub melee: Melee,
    pub grenade: Grenade,
}

impl AbilitiesTrait for Abilities {
    fn super_(&self) -> Box<dyn Display> {
        Box::new(self.super_)
    }

    fn class(&self) -> Box<dyn Display> {
        Box::new(self.class)
    }

    fn jump(&self) -> Box<dyn Display> {
        Box::new(self.jump)
    }

    fn melee(&self) -> Box<dyn Display> {
        Box::new(self.melee)
    }

    fn grenade(&self) -> Box<dyn Display> {
        Box::new(self.grenade)
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Super {
    SongOfFlame,
    NovaBombCataclysm,
    Stormtrance,
    WintersWrath,
    Needlestorm,
}

impl Display for Super {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::SongOfFlame => "Song of Flame",
            Self::NovaBombCataclysm => "Nova Bomb: Cataclysm",
            Self::Stormtrance => "Stormtrance",
            Self::WintersWrath => "Winter's Wrath",
            Self::Needlestorm => "Needlestorm",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Melee {
    ArcaneNeedle,
    PocketSingularity,
    IncineratorSnap,
    ChainLightning,
    PenumbralBlast,
}

impl Display for Melee {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::ArcaneNeedle => "arcane_needle",
            Self::PocketSingularity => "pocket_singularity",
            Self::IncineratorSnap => "incinerator_snap",
            Self::ChainLightning => "chain_lightning",
            Self::PenumbralBlast => "Penumbral Blast",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Grenade {
    VortexGrenade,
    HealingGrenade,
    StormGrenade,
    ColdsnapGrenade,
    ThreadlingGrenade,
}

impl Display for Grenade {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::VortexGrenade => "vortex_grenade",
            Self::HealingGrenade => "healing_grenade",
            Self::StormGrenade => "storm_grenade",
            Self::ColdsnapGrenade => "coldsnap_grenade",
            Self::ThreadlingGrenade => "threadling_grenade",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Aspect {
    Hellion([PrismaticFragment; 2]),
    FeedTheVoid([PrismaticFragment; 2]),
    LightningSurge([PrismaticFragment; 3]),
    BleakWatcher([PrismaticFragment; 2]),
    WeaversCall([PrismaticFragment; 3]),
}

impl AspectTrait for Aspect {
    fn fragments(&self) -> [Option<Box<dyn Display>>; 3] {
        match *self {
            Self::Hellion(fragments)
            | Self::FeedTheVoid(fragments)
            | Self::BleakWatcher(fragments) => [
                Some(box_display(fragments[0])),
                Some(box_display(fragments[1])),
                None,
            ],
            Self::LightningSurge(fragments) | Self::WeaversCall(fragments) => [
                Some(box_display(fragments[0])),
                Some(box_display(fragments[1])),
                Some(box_display(fragments[2])),
            ],
        }
    }
}

impl Display for Aspect {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Hellion(_) => "hellion",
            Self::FeedTheVoid(_) => "feed_the_void",
            Self::LightningSurge(_) => "lightning_surge",
            Self::BleakWatcher(_) => "bleak_watcher",
            Self::WeaversCall(_) => "weavers_call",
        };

        write!(f, "{s}")
    }
}
