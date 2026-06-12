use std::fmt;
use std::fmt::{Display, Formatter};

use super::super::{
    Abilities as AbilitiesTrait,
    Aspect as AspectTrait,
    SolarFragment,
    SolarGrenade,
};
use super::{ClassAbility, Jump};

#[derive(Clone, Copy)]
pub struct Abilities {
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

#[derive(Clone, Copy)]
pub enum Super {
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

#[derive(Clone, Copy)]
pub enum Melee {
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

#[derive(Clone, Copy)]
pub enum Aspect {
    RoaringFlames([SolarFragment; 2]),
    SolInvictus([SolarFragment; 2]),
    Consecration([SolarFragment; 3]),
    Shieldburst([SolarFragment; 3]),
}

impl AspectTrait for Aspect {
    fn fragments(&self) -> [Option<Box<dyn Display>>; 3] {
        match *self {
            Self::RoaringFlames(fragments) | Self::SolInvictus(fragments) => [
                Some(Box::new(fragments[0]) as Box<dyn Display>),
                Some(Box::new(fragments[1]) as Box<dyn Display>),
                None,
            ],
            Self::Consecration(fragments) | Self::Shieldburst(fragments) => [
                Some(Box::new(fragments[0]) as Box<dyn Display>),
                Some(Box::new(fragments[1]) as Box<dyn Display>),
                Some(Box::new(fragments[2]) as Box<dyn Display>),
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
