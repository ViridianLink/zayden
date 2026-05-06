use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub enum Affinity {
    None,
    Kinetic,
    Arc,
    Void,
    Solar,
    Stasis,
    Strand,
}

impl FromStr for Affinity {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "" => Ok(Affinity::None),
            "Kinetic" => Ok(Affinity::Kinetic),
            "Arc" => Ok(Affinity::Arc),
            "Void" => Ok(Affinity::Void),
            "Solar" => Ok(Affinity::Solar),
            "Stasis" => Ok(Affinity::Stasis),
            "Strand" => Ok(Affinity::Strand),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Affinity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Affinity::None => write!(f, ""),
            Affinity::Kinetic => write!(f, "Kinetic"),
            Affinity::Arc => write!(f, "Arc"),
            Affinity::Void => write!(f, "Void"),
            Affinity::Solar => write!(f, "Solar"),
            Affinity::Stasis => write!(f, "Stasis"),
            Affinity::Strand => write!(f, "Strand"),
        }
    }
}
