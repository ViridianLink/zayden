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
    StormsEdge,
    ShadowshotDeadfall,
    GoldenGunMarksman,
    SilenceAndSquall,
    Silkstrike,
}

impl Display for Super {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::StormsEdge => "Storm's Edge",
            Self::ShadowshotDeadfall => "Shadowshot: Deadfall",
            Self::GoldenGunMarksman => "Golden Gun: Marksman",
            Self::SilenceAndSquall => "Silence and Squall",
            Self::Silkstrike => "Silkstrike",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Melee {
    WitheringBlade,
    SnareBomb,
    KnifeTrick,
    CombinationBlow,
    ThreadedSpike,
}

impl Display for Melee {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::WitheringBlade => "withering_blade",
            Self::SnareBomb => "snare_bomb",
            Self::KnifeTrick => "knife_trick",
            Self::CombinationBlow => "combination_blow",
            Self::ThreadedSpike => "threaded_spike",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Grenade {
    Grapple,
    MagneticGrenade,
    SwarmGrenade,
    ArcboltGrenade,
    DuskfieldGrenade,
}

impl Display for Grenade {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Grapple => "grapple",
            Self::MagneticGrenade => "magnetic_grenade",
            Self::SwarmGrenade => "swarm_grenade",
            Self::ArcboltGrenade => "arcbolt_grenade",
            Self::DuskfieldGrenade => "duskfield_grenade",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Aspect {
    Ascension([PrismaticFragment; 2]),
    StylishExecutioner([PrismaticFragment; 2]),
    GunpowderGamble([PrismaticFragment; 3]),
    WintersShroud([PrismaticFragment; 2]),
    ThreadedSpecter([PrismaticFragment; 3]),
}

impl AspectTrait for Aspect {
    fn fragments(&self) -> [Option<Box<dyn Display>>; 3] {
        match *self {
            Self::Ascension(fragments)
            | Self::StylishExecutioner(fragments)
            | Self::WintersShroud(fragments) => [
                Some(box_display(fragments[0])),
                Some(box_display(fragments[1])),
                None,
            ],
            Self::GunpowderGamble(fragments) | Self::ThreadedSpecter(fragments) => [
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
            Self::Ascension(_) => "ascension",
            Self::StylishExecutioner(_) => "stylish_executioner",
            Self::GunpowderGamble(_) => "gunpowder_gamble",
            Self::WintersShroud(_) => "winters_shroud",
            Self::ThreadedSpecter(_) => "threaded_specter",
        };

        write!(f, "{s}")
    }
}
