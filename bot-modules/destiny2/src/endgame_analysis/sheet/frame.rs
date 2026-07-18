use std::fmt;
use std::str::FromStr;

use tracing::error;

#[derive(Debug)]
pub enum Frame {
    Rapid,
    RapidSlug,
    PinpointSlug,
    Aggressive,
    Lightweight,
    HeavyBurst,
    Precision,
    Adaptive,
    HighImpact,
    AreaDenial,
    MicroMissile,
    DoubleFire,
    Wave,
    CompressedWave,
    Vortex,
    Caster,
    AdaptiveBurst,
    Support,
    AggressiveBurst,
    LegacyPR55,
    TogetherForever,
    MIDASynergy,
    HighImpactLongBow,
    SpreadShot,
    RocketAssisted,
    Balanced,
    Dynamic,
    Disruption,
    ShotPackage,
}

impl FromStr for Frame {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Rapid" => Ok(Self::Rapid),
            "Rapid Slug" => Ok(Self::RapidSlug),
            "Pinpoint Slug" => Ok(Self::PinpointSlug),
            "Aggressive" => Ok(Self::Aggressive),
            "Lightweight" => Ok(Self::Lightweight),
            "Heavy Burst" => Ok(Self::HeavyBurst),
            "Precision" => Ok(Self::Precision),
            "Adaptive" => Ok(Self::Adaptive),
            "High-Impact" => Ok(Self::HighImpact),
            "Area Denial" => Ok(Self::AreaDenial),
            "Micro-Missile" => Ok(Self::MicroMissile),
            "Double Fire" => Ok(Self::DoubleFire),
            "Wave" => Ok(Self::Wave),
            "Compressed Wave" => Ok(Self::CompressedWave),
            "Vortex" => Ok(Self::Vortex),
            "Caster" => Ok(Self::Caster),
            "Adaptive Burst" => Ok(Self::AdaptiveBurst),
            "Support" => Ok(Self::Support),
            "Aggressive Burst" => Ok(Self::AggressiveBurst),
            "Legacy PR-55" => Ok(Self::LegacyPR55),
            "Together Forever" => Ok(Self::TogetherForever),
            "MIDA Synergy" => Ok(Self::MIDASynergy),
            "High-Impact Longbow" => Ok(Self::HighImpactLongBow),
            "Spread Shot" => Ok(Self::SpreadShot),
            "Rocket-Assisted" => Ok(Self::RocketAssisted),
            "Balanced" | "Balanced (260RPM)" | "Balanced (450RPM)"
            | "Balanced (540RPM)" | "Balanced (900RPM)" => Ok(Self::Balanced),
            "Dynamic" | "Dynamic (140RPM)" | "Dynamic (180RPM)"
            | "Dynamic (360RPM)" | "Dynamic (540RPM)" => Ok(Self::Dynamic),
            "Disruption" => Ok(Self::Disruption),
            "Shot Package" => Ok(Self::ShotPackage),
            _ => {
                error!("Failed to parse: '{s}'");
                Err(())
            },
        }
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rapid => write!(f, "Rapid"),
            Self::RapidSlug => write!(f, "Rapid Slug"),
            Self::PinpointSlug => write!(f, "Pinpoint Slug"),
            Self::Aggressive => write!(f, "Aggressive"),
            Self::Lightweight => write!(f, "Lightweight"),
            Self::HeavyBurst => write!(f, "Heavy Burst"),
            Self::Precision => write!(f, "Precision"),
            Self::Adaptive => write!(f, "Adaptive"),
            Self::HighImpact => write!(f, "High-Impact"),
            Self::AreaDenial => write!(f, "Area Denial"),
            Self::MicroMissile => write!(f, "Micro-Missile"),
            Self::DoubleFire => write!(f, "Double Fire"),
            Self::Wave => write!(f, "Wave"),
            Self::CompressedWave => write!(f, "Compressed Wave"),
            Self::Vortex => write!(f, "Vortex"),
            Self::Caster => write!(f, "Caster"),
            Self::AdaptiveBurst => write!(f, "Adaptive Burst"),
            Self::Support => write!(f, "Support"),
            Self::AggressiveBurst => write!(f, "Aggressive Burst"),
            Self::LegacyPR55 => write!(f, "Legacy PR-55"),
            Self::TogetherForever => write!(f, "Together Forever"),
            Self::MIDASynergy => write!(f, "MIDA Synergy"),
            Self::HighImpactLongBow => write!(f, "High-Impact Longbow"),
            Self::SpreadShot => write!(f, "Spread Shot"),
            Self::RocketAssisted => write!(f, "Rocket-Assisted"),
            Self::Balanced => write!(f, "Balanced"),
            Self::Dynamic => write!(f, "Dynamic"),
            Self::Disruption => write!(f, "Disruption"),
            Self::ShotPackage => write!(f, "Shot Package"),
        }
    }
}
