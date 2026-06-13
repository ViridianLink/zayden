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
    GoldenGunDeadshot,
    GoldenGunMarksman,
    BladeBarrage,
}

impl Display for Super {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::GoldenGunDeadshot => "Golden Gun: Deadshot",
            Self::GoldenGunMarksman => "Golden Gun: Marksman",
            Self::BladeBarrage => "Blade Barrage",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Melee {
    LightweightKnife,
    WeightedThrowingKnife,
    KnifeTrick,
    ProximityExplosiveKnife,
}

impl Display for Melee {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::LightweightKnife => "lightweight_knife",
            Self::WeightedThrowingKnife => "weighted_throwing_knife",
            Self::KnifeTrick => "knife_trick",
            Self::ProximityExplosiveKnife => "proximity_explosive_knife",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(crate) enum Aspect {
    KnockEmDown([SolarFragment; 2]),
    OnYourMark([SolarFragment; 3]),
    GunpowderGamble([SolarFragment; 2]),
    Crackshot([SolarFragment; 2]),
}

impl AspectTrait for Aspect {
    fn fragments(&self) -> [Option<Box<dyn Display>>; 3] {
        match *self {
            Self::GunpowderGamble(fragments)
            | Self::KnockEmDown(fragments)
            | Self::Crackshot(fragments) => [
                Some(box_display(fragments[0])),
                Some(box_display(fragments[1])),
                None,
            ],
            Self::OnYourMark(fragments) => [
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
            Self::KnockEmDown(_) => "knock_em_down",
            Self::OnYourMark(_) => "on_your_mark",
            Self::GunpowderGamble(_) => "gunpowder_gamble",
            Self::Crackshot(_) => "crackshot",
        };

        write!(f, "{s}")
    }
}
