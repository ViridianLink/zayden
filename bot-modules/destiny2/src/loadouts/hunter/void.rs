use std::fmt;
use std::fmt::{Display, Formatter};

use super::super::{
    Abilities as AbilitiesTrait,
    Aspect as AspectTrait,
    VoidFragment,
    VoidGrenade,
    box_display,
};
use super::{ClassAbility, Jump};

#[derive(Clone, Copy)]
pub(crate) struct Abilities {
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

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Super {
    ShadowshotDeadfall,
    SpectralBlades,
    ShadowshotMoebiusQuiver,
}

impl Display for Super {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::ShadowshotDeadfall => "Shadowshot: Deadfall",
            Self::SpectralBlades => "Spectral Blades",
            Self::ShadowshotMoebiusQuiver => "Shadowshot: Moebius Quiver",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Melee {
    SnareBomb,
    PhantomSurge,
}

impl Display for Melee {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::SnareBomb => "snare_bomb",
            Self::PhantomSurge => "phantom_surge",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Aspect {
    TrappersAmbush([VoidFragment; 2]),
    VanishingStep([VoidFragment; 2]),
    StylishExecutioner([VoidFragment; 2]),
    OnTheProwl([VoidFragment; 3]),
}

impl AspectTrait for Aspect {
    fn fragments(&self) -> [Option<Box<dyn Display>>; 3] {
        match *self {
            Self::VanishingStep(fragments)
            | Self::StylishExecutioner(fragments)
            | Self::TrappersAmbush(fragments) => [
                Some(box_display(fragments[0])),
                Some(box_display(fragments[1])),
                None,
            ],
            Self::OnTheProwl(fragments) => [
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
            Self::TrappersAmbush(_) => "trappers_ambush",
            Self::VanishingStep(_) => "vanishing_step",
            Self::StylishExecutioner(_) => "stylish_executioner",
            Self::OnTheProwl(_) => "on_the_prowl",
        };

        write!(f, "{s}")
    }
}
