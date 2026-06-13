use std::fmt;
use std::fmt::{Display, Formatter};

use super::super::{
    Abilities as AbilitiesTrait,
    Aspect as AspectTrait,
    StrandFragment,
    StrandGrenade,
    box_display,
};
use super::{ClassAbility, Jump};

#[derive(Clone, Copy)]
pub(crate) struct Abilities {
    pub super_: Super,
    pub class: ClassAbility,
    pub jump: Jump,
    pub melee: Melee,
    pub grenade: StrandGrenade,
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
    Silkstrike,
}

impl Display for Super {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Silkstrike => "Silkstrike",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Melee {
    ThreadedSpike,
}

impl Display for Melee {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::ThreadedSpike => "threaded_spike",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Aspect {
    WidowsSilk([StrandFragment; 3]),
    EnsnaringSlam([StrandFragment; 2]),
    ThreadedSpecter([StrandFragment; 3]),
    WhirlingMaelstrom([StrandFragment; 2]),
}

impl AspectTrait for Aspect {
    fn fragments(&self) -> [Option<Box<dyn Display>>; 3] {
        match *self {
            Self::EnsnaringSlam(fragments) | Self::WhirlingMaelstrom(fragments) => [
                Some(box_display(fragments[0])),
                Some(box_display(fragments[1])),
                None,
            ],
            Self::WidowsSilk(fragments) | Self::ThreadedSpecter(fragments) => [
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
            Self::WidowsSilk(_) => "widows_silk",
            Self::EnsnaringSlam(_) => "ensnaring_slam",
            Self::ThreadedSpecter(_) => "threaded_specter",
            Self::WhirlingMaelstrom(_) => "whirling_maelstrom",
        };

        write!(f, "{s}")
    }
}
