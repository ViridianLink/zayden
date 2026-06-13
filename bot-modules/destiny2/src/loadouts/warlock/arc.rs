use std::fmt;
use std::fmt::{Display, Formatter};

use super::super::{
    Abilities as AbilitiesTrait,
    ArcFragment,
    ArcGrenade,
    Aspect as AspectTrait,
    box_display,
};
use super::{ClassAbility, Jump};

#[derive(Clone, Copy)]
pub(crate) struct Abilities {
    pub super_: Super,
    pub class: ClassAbility,
    pub jump: Jump,
    pub melee: Melee,
    pub grenade: ArcGrenade,
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
    Stormtrance,
    ChaosReach,
}

impl Display for Super {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Stormtrance => "Stormtrance",
            Self::ChaosReach => "Chaos Reach",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Melee {
    BallLightning,
    ChainLightning,
}

impl Display for Melee {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::BallLightning => "ball_lightning",
            Self::ChainLightning => "chain_lightning",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Aspect {
    ArcSoul([ArcFragment; 2]),
    LightningSurge([ArcFragment; 3]),
    ElectrostaticMind([ArcFragment; 2]),
    IonicSentry([ArcFragment; 2]),
}

impl AspectTrait for Aspect {
    fn fragments(&self) -> [Option<Box<dyn Display>>; 3] {
        match *self {
            Self::ArcSoul(fragments)
            | Self::ElectrostaticMind(fragments)
            | Self::IonicSentry(fragments) => [
                Some(box_display(fragments[0])),
                Some(box_display(fragments[1])),
                None,
            ],
            Self::LightningSurge(fragments) => [
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
            Self::ArcSoul(_) => "arc_soul",
            Self::LightningSurge(_) => "lightning_surge",
            Self::ElectrostaticMind(_) => "electrostatic_mind",
            Self::IonicSentry(_) => "ionic_sentry",
        };

        write!(f, "{s}")
    }
}
