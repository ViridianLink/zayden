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

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Melee {
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

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Aspect {
    TouchOfThunder([ArcFragment; 2]),
    Juggernaut([ArcFragment; 2]),
    Knockout([ArcFragment; 2]),
    StormsKeep([ArcFragment; 2]),
}

impl AspectTrait for Aspect {
    fn fragments(&self) -> [Option<Box<dyn Display>>; 3] {
        match *self {
            Self::TouchOfThunder(fragments)
            | Self::Juggernaut(fragments)
            | Self::Knockout(fragments)
            | Self::StormsKeep(fragments) => [
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
            Self::TouchOfThunder(_) => "touch_of_thunder",
            Self::Juggernaut(_) => "juggernaut",
            Self::Knockout(_) => "knockout",
            Self::StormsKeep(_) => "storms_keep",
        };

        write!(f, "{s}")
    }
}
