use std::fmt;
use std::fmt::{Display, Formatter};

use super::super::{
    Abilities as AbilitiesTrait,
    Aspect as AspectTrait,
    StasisFragment,
    StasisGrenade,
    box_display,
};
use super::{ClassAbility, Jump};

#[derive(Clone, Copy)]
pub(crate) struct Abilities {
    pub super_: Super,
    pub class: ClassAbility,
    pub jump: Jump,
    pub melee: Melee,
    pub grenade: StasisGrenade,
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
pub(crate) enum Super {
    SilenceAndSquall,
}

impl Display for Super {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::SilenceAndSquall => "Silence and Squall",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Melee {
    WitheringBlade,
}

impl Display for Melee {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::WitheringBlade => "withering_blade",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Aspect {
    WintersShroud([StasisFragment; 3]),
    Shatterdive([StasisFragment; 3]),
    GrimHarvest([StasisFragment; 2]),
    TouchOfWinter([StasisFragment; 2]),
}

impl AspectTrait for Aspect {
    fn fragments(&self) -> [Option<Box<dyn Display>>; 3] {
        match *self {
            Self::GrimHarvest(fragments) | Self::TouchOfWinter(fragments) => [
                Some(box_display(fragments[0])),
                Some(box_display(fragments[1])),
                None,
            ],
            Self::WintersShroud(fragments) | Self::Shatterdive(fragments) => [
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
            Self::WintersShroud(_) => "winters_shroud",
            Self::Shatterdive(_) => "shatterdive",
            Self::GrimHarvest(_) => "grim_harvest",
            Self::TouchOfWinter(_) => "touch_of_winter",
        };

        write!(f, "{s}")
    }
}
