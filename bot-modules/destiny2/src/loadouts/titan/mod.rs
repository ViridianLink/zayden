pub mod arc;
mod armour;
pub mod prismatic;
pub mod solar;
pub mod stasis;
pub mod strand;
pub mod void;

use std::fmt;
use std::fmt::{Display, Formatter};

pub use armour::{Gauntlets, Greaves, Helmet, Mark, Plate};

use super::{Abilities, Aspect, Subclass as SubclassTrait};

#[derive(Clone, Copy)]
pub enum Subclass {
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
            Self::Prismatic { aspects, .. } => {
                aspects.map(|a| Box::new(a) as Box<dyn Aspect>)
            },
            Self::Arc { aspects, .. } => {
                aspects.map(|a| Box::new(a) as Box<dyn Aspect>)
            },
            Self::Solar { aspects, .. } => {
                aspects.map(|a| Box::new(a) as Box<dyn Aspect>)
            },
            Self::Void { aspects, .. } => {
                aspects.map(|a| Box::new(a) as Box<dyn Aspect>)
            },
            Self::Stasis { aspects, .. } => {
                aspects.map(|a| Box::new(a) as Box<dyn Aspect>)
            },
            Self::Strand { aspects, .. } => {
                aspects.map(|a| Box::new(a) as Box<dyn Aspect>)
            },
        }
    }
}

impl Display for Subclass {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Arc { .. } => "Arc",
            Self::Void { .. } => "Void",
            Self::Strand { .. } => "Strand",
            Self::Stasis { .. } => "Stasis",
            Self::Solar { .. } => "Solar",
            Self::Prismatic { .. } => "Prismatic",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum ClassAbility {
    ToweringBarricade,
    RallyBarricade,
    Thruster,
}

impl Display for ClassAbility {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::ToweringBarricade => "towering_barricade",
            Self::RallyBarricade => "rally_barricade",
            Self::Thruster => "thruster",
        };

        write!(f, "{name}")
    }
}

#[derive(Clone, Copy)]
pub enum Jump {
    HighLift,
    StrafeLift,
    CatapultLift,
}

impl Display for Jump {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::HighLift => "high_lift",
            Self::StrafeLift => "strafe_lift",
            Self::CatapultLift => "catapult_lift",
        };

        write!(f, "{name}")
    }
}
