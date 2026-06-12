use std::fmt;
use std::fmt::{Display, Formatter};

use super::super::{
    Abilities as AbilitiesTrait,
    Aspect as AspectTrait,
    VoidFragment,
    VoidGrenade,
};
use super::{ClassAbility, Jump};

#[derive(Clone, Copy)]
pub struct Abilities {
    pub super_: Super,
    pub class: ClassAbility,
    pub jump: Jump,
    pub melee: Melee,
    pub grenade: VoidGrenade,
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
    WardOfDawn,
    SentinelShield,
    TwilightArsenal,
}

impl Display for Super {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::WardOfDawn => "Ward of Dawn",
            Self::SentinelShield => "Sentinel Shield",
            Self::TwilightArsenal => "Twilight Arsenal",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Melee {
    ShieldBash,
    ShieldThrow,
}

impl Display for Melee {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::ShieldBash => "Shield Bash",
            Self::ShieldThrow => "Shield Throw",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum Aspect {
    ControlledDemolition([VoidFragment; 2]),
    Bastion([VoidFragment; 3]),
    OffensiveBulwark([VoidFragment; 2]),
    Unbreakable([VoidFragment; 3]),
}

impl AspectTrait for Aspect {
    fn fragments(&self) -> [Option<Box<dyn Display>>; 3] {
        match *self {
            Self::ControlledDemolition(fragments)
            | Self::OffensiveBulwark(fragments) => [
                Some(Box::new(fragments[0]) as Box<dyn Display>),
                Some(Box::new(fragments[1]) as Box<dyn Display>),
                None,
            ],
            Self::Bastion(fragments) | Self::Unbreakable(fragments) => [
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
            Self::ControlledDemolition(_) => "controlled_demolition",
            Self::Bastion(_) => "bastion",
            Self::OffensiveBulwark(_) => "offensive_bulwark",
            Self::Unbreakable(_) => "unbreakable",
        };

        write!(f, "{s}")
    }
}
