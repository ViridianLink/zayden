use std::fmt;
use std::fmt::{Display, Formatter};

use super::{Subclass, titan};

#[derive(Clone, Copy)]
pub enum DestinyClass {
    Warlock(titan::Subclass),
    Titan(titan::Subclass),
    Hunter(titan::Subclass),
}

impl DestinyClass {
    pub fn subclass(self) -> Box<dyn Subclass> {
        match self {
            Self::Warlock(subclass) => Box::new(subclass),
            Self::Titan(subclass) => Box::new(subclass),
            Self::Hunter(subclass) => Box::new(subclass),
        }
    }
}

impl Display for DestinyClass {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Warlock(_) => write!(f, "Warlock"),
            Self::Titan(_) => write!(f, "Titan"),
            Self::Hunter(_) => write!(f, "Hunter"),
        }
    }
}
