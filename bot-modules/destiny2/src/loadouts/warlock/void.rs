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
    Warp,
    Vortex,
    Cataclysm,
}

impl Display for Super {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Warp => "Nova Warp",
            Self::Vortex => "Nova Bomb: Vortex",
            Self::Cataclysm => "Nova Bomb: Cataclysm",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Melee {
    PocketSingularity,
}

impl Display for Melee {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::PocketSingularity => "pocket_singularity",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Aspect {
    ChaosAccelerant([VoidFragment; 2]),
    FeedTheVoid([VoidFragment; 2]),
    ChildOfTheOldGods([VoidFragment; 3]),
    SoulSiphon([VoidFragment; 3]),
}

impl AspectTrait for Aspect {
    fn fragments(&self) -> [Option<Box<dyn Display>>; 3] {
        match *self {
            Self::ChaosAccelerant(fragments) | Self::FeedTheVoid(fragments) => [
                Some(box_display(fragments[0])),
                Some(box_display(fragments[1])),
                None,
            ],
            Self::ChildOfTheOldGods(fragments) | Self::SoulSiphon(fragments) => [
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
            Self::ChaosAccelerant(_) => "chaos_accelerant",
            Self::FeedTheVoid(_) => "feed_the_void",
            Self::ChildOfTheOldGods(_) => "child_of_the_old_gods",
            Self::SoulSiphon(_) => "soul_siphon",
        };

        write!(f, "{s}")
    }
}
