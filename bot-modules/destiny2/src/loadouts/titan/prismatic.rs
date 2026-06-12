use std::fmt;
use std::fmt::{Display, Formatter};

use super::super::fragments::VoidFragment;
use super::{
    Abilities as AbilitiesTrait,
    Aspect as AspectTrait,
    ClassAbility,
    Jump,
};

#[derive(Clone, Copy)]
pub struct Abilities {
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

#[derive(Clone, Copy)]
pub enum Super {
    FistsOfHavoc,
    Thundercrash,
}

impl Display for Super {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::FistsOfHavoc => "Fists of Havoc",
            Self::Thundercrash => "Thundercrash",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Melee {
    SeismicStrike,
    BallisticSlam,
    Thunderclap,
}

impl Display for Melee {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::SeismicStrike => "Seismic Strike",
            Self::BallisticSlam => "Ballistic Slam",
            Self::Thunderclap => "Thunderclap",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Grenade {
    LightningGrenade,
    StormGrenade,
    FlashbangGrenade,
    PulseGrenade,
    SkipGrenade,
    FluxGrenade,
    ArcboltGrenade,
}

impl Display for Grenade {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::LightningGrenade => "Lightning Grenade",
            Self::StormGrenade => "Storm Grenade",
            Self::FlashbangGrenade => "Flashbang Grenade",
            Self::PulseGrenade => "Pulse Grenade",
            Self::SkipGrenade => "Skip Grenade",
            Self::FluxGrenade => "Flux Grenade",
            Self::ArcboltGrenade => "Arcbolt Grenade",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Aspect {
    TouchOfThunder([VoidFragment; 2]),
    Juggernaut([VoidFragment; 2]),
    Knockout([VoidFragment; 2]),
    StormsKeep([VoidFragment; 2]),
}

impl AspectTrait for Aspect {
    fn fragments(&self) -> [Option<Box<dyn Display>>; 3] {
        match *self {
            Self::TouchOfThunder(fragments)
            | Self::Juggernaut(fragments)
            | Self::Knockout(fragments)
            | Self::StormsKeep(fragments) => [
                Some(Box::new(fragments[0]) as Box<dyn Display>),
                Some(Box::new(fragments[1]) as Box<dyn Display>),
                None,
            ],
        }
    }
}

impl Display for Aspect {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::TouchOfThunder(_) => "touch_of_thunder",
            Self::Juggernaut(_) => "juggernaut",
            Self::Knockout(_) => "knockout",
            Self::StormsKeep(_) => "storms_keep",
        };

        write!(f, "{s}")
    }
}
