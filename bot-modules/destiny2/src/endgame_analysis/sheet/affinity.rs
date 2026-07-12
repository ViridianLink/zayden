use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, sqlx::Type)]
#[sqlx(type_name = "destiny2_affinity", rename_all = "lowercase")]
pub enum Affinity {
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
            "Kinetic" => Ok(Self::Kinetic),
            "Arc" => Ok(Self::Arc),
            "Void" => Ok(Self::Void),
            "Solar" => Ok(Self::Solar),
            "Stasis" => Ok(Self::Stasis),
            "Strand" => Ok(Self::Strand),
            _ => Err(()),
        }
    }
}

impl fmt::Display for Affinity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Kinetic => write!(f, "Kinetic"),
            Self::Arc => write!(f, "Arc"),
            Self::Void => write!(f, "Void"),
            Self::Solar => write!(f, "Solar"),
            Self::Stasis => write!(f, "Stasis"),
            Self::Strand => write!(f, "Strand"),
        }
    }
}
