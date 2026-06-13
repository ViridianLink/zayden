use std::fmt;
use std::fmt::{Display, Formatter};

use super::super::{
    Abilities as AbilitiesTrait,
    Aspect as AspectTrait,
    SolarFragment,
    SolarGrenade,
    box_display,
};
use super::{ClassAbility, Jump};

#[derive(Clone, Copy)]
pub(crate) struct Abilities {
    pub super_: Super,
    pub class: ClassAbility,
    pub jump: Jump,
    pub melee: Melee,
    pub grenade: SolarGrenade,
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
    HammerOfSol,
    BurningMaul,
}

impl Display for Super {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::HammerOfSol => "Hammer of Sol",
            Self::BurningMaul => "Burning Maul",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Melee {
    HammerStrike,
    ThrowingHammer,
}

impl Display for Melee {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::HammerStrike => "Hammer Strike",
            Self::ThrowingHammer => "Throwing Hammer",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Aspect {
    RoaringFlames([SolarFragment; 2]),
    SolInvictus([SolarFragment; 2]),
    Consecration([SolarFragment; 3]),
    Shieldburst([SolarFragment; 3]),
}

impl AspectTrait for Aspect {
    fn fragments(&self) -> [Option<Box<dyn Display>>; 3] {
        match *self {
            Self::RoaringFlames(fragments) | Self::SolInvictus(fragments) => [
                Some(box_display(fragments[0])),
                Some(box_display(fragments[1])),
                None,
            ],
            Self::Consecration(fragments) | Self::Shieldburst(fragments) => [
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
            Self::RoaringFlames(_) => "roaring_flames",
            Self::SolInvictus(_) => "sol_invictus",
            Self::Consecration(_) => "consecration",
            Self::Shieldburst(_) => "shieldburst",
        };

        write!(f, "{s}")
    }
}
