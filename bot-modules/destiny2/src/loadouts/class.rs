use std::fmt;
use std::fmt::{Display, Formatter};

use super::{Subclass, hunter, titan, warlock};

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum DestinyClass {
    Warlock(warlock::Subclass),
    Titan(titan::Subclass),
    Hunter(hunter::Subclass),
}

impl DestinyClass {
    pub(super) fn subclass(self) -> Box<dyn Subclass> {
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
