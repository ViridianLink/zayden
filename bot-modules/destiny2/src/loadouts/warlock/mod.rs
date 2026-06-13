pub(super) mod arc;
mod armour;
pub(super) mod prismatic;
pub(super) mod solar;
pub(super) mod stasis;
pub(super) mod strand;
pub(super) mod void;

use std::fmt;
use std::fmt::{Display, Formatter};

use super::{Abilities, Aspect, Subclass as SubclassTrait, box_aspect};

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum Subclass {
    Prismatic { abilities: prismatic::Abilities, aspects: [prismatic::Aspect; 2] },
    Arc { abilities: arc::Abilities, aspects: [arc::Aspect; 2] },
    Solar { abilities: solar::Abilities, aspects: [solar::Aspect; 2] },
    Void { abilities: void::Abilities, aspects: [void::Aspect; 2] },
    Stasis { abilities: stasis::Abilities, aspects: [stasis::Aspect; 2] },
    Strand { abilities: strand::Abilities, aspects: [strand::Aspect; 2] },
}

impl SubclassTrait for Subclass {
    fn abilities(&self) -> Box<dyn Abilities> {
        match self {
            Self::Prismatic { abilities, .. } => Box::new(*abilities),
            Self::Arc { abilities, .. } => Box::new(*abilities),
            Self::Solar { abilities, .. } => Box::new(*abilities),
            Self::Void { abilities, .. } => Box::new(*abilities),
            Self::Stasis { abilities, .. } => Box::new(*abilities),
            Self::Strand { abilities, .. } => Box::new(*abilities),
        }
    }

    fn aspects(&self) -> [Box<dyn Aspect>; 2] {
        match self {
            Self::Prismatic { aspects, .. } => aspects.map(box_aspect),
            Self::Arc { aspects, .. } => aspects.map(box_aspect),
            Self::Solar { aspects, .. } => aspects.map(box_aspect),
            Self::Void { aspects, .. } => aspects.map(box_aspect),
            Self::Stasis { aspects, .. } => aspects.map(box_aspect),
            Self::Strand { aspects, .. } => aspects.map(box_aspect),
        }
    }
}

impl Display for Subclass {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Prismatic { .. } => "Prismatic",
            Self::Arc { .. } => "Arc",
            Self::Solar { .. } => "Solar",
            Self::Void { .. } => "Void",
            Self::Stasis { .. } => "Stasis",
            Self::Strand { .. } => "Strand",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum ClassAbility {
    HealingRift,
    EmpoweringRift,
    PhoenixDive,
}

impl Display for ClassAbility {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::HealingRift => "healing_rift",
            Self::EmpoweringRift => "empowering_rift",
            Self::PhoenixDive => "phoenix_dive",
        };

        write!(f, "{name}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum Jump {
    StrafeJump,
    BurstJump,
    BalancedJump,
    Blink,
}

impl Display for Jump {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::StrafeJump => "strafe_glide",
            Self::BurstJump => "burst_glide",
            Self::BalancedJump => "balanced_glide",
            Self::Blink => "blink",
        };

        write!(f, "{name}")
    }
}
