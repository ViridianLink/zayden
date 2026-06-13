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
    TwilightArsenal,
    HammerOfSol,
    Thundercrash,
    GlacialQuake,
    Bladefury,
}

impl Display for Super {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::TwilightArsenal => "Twilight Arsenal",
            Self::HammerOfSol => "Hammer of Sol",
            Self::Thundercrash => "Thundercrash",
            Self::GlacialQuake => "Glacial Quake",
            Self::Bladefury => "Bladefury",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Melee {
    SheildThrow,
    ShiverStrike,
    HammerStrike,
    Thunderclap,
    FrenziedBlade,
}

impl Display for Melee {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::SheildThrow => "shield_throw",
            Self::ShiverStrike => "shiver_strike",
            Self::HammerStrike => "hammer_strike",
            Self::Thunderclap => "thunderclap",
            Self::FrenziedBlade => "frenzied_blade",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Grenade {
    Glacier,
    Pulse,
    Thermite,
    Suppressor,
    Shackle,
}

impl Display for Grenade {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Glacier => "glacier_grenade",
            Self::Pulse => "pulse_grenade",
            Self::Thermite => "thermite_grenade",
            Self::Suppressor => "suppressor_grenade",
            Self::Shackle => "shackle_grenade",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Aspect {
    Unbreakable([PrismaticFragment; 3]),
    Consecration([PrismaticFragment; 2]),
    Knockout([PrismaticFragment; 2]),
    DiamondLance([PrismaticFragment; 3]),
    DrengrsLash([PrismaticFragment; 3]),
}

impl AspectTrait for Aspect {
    fn fragments(&self) -> [Option<Box<dyn Display>>; 3] {
        match *self {
            Self::Unbreakable(fragments)
            | Self::DiamondLance(fragments)
            | Self::DrengrsLash(fragments) => [
                Some(box_display(fragments[0])),
                Some(box_display(fragments[1])),
                Some(box_display(fragments[2])),
            ],
            Self::Knockout(fragments) | Self::Consecration(fragments) => [
                Some(box_display(fragments[0])),
                Some(box_display(fragments[1])),
                None,
            ],
        }
    }
}

impl Display for Aspect {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Unbreakable(_) => "unbreakable",
            Self::Consecration(_) => "consecration",
            Self::Knockout(_) => "knockout",
            Self::DiamondLance(_) => "diamond_lance",
            Self::DrengrsLash(_) => "drengrs_lash",
        };

        write!(f, "{s}")
    }
}
