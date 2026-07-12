use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use tracing::error;

#[derive(Debug, Deserialize, Serialize)]
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
    BalancedHeat(u16),
    DynamicHeat(u16),
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
            "Balanced (260RPM)" => Ok(Self::BalancedHeat(260)),
            "Balanced (450RPM)" => Ok(Self::BalancedHeat(450)),
            "Balanced (540RPM)" => Ok(Self::BalancedHeat(540)),
            "Balanced (900RPM)" => Ok(Self::BalancedHeat(900)),
            "Dynamic (140RPM)" => Ok(Self::DynamicHeat(140)),
            "Dynamic (180RPM)" => Ok(Self::DynamicHeat(180)),
            "Dynamic (360RPM)" => Ok(Self::DynamicHeat(360)),
            "Dynamic (540RPM)" => Ok(Self::DynamicHeat(540)),
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
            Self::BalancedHeat(rpm) => write!(f, "Balanced Heat ({rpm}RPM)"),
            Self::DynamicHeat(rpm) => write!(f, "Dynamic Heat ({rpm}RPM)"),
            Self::Disruption => write!(f, "Disruption"),
            Self::ShotPackage => write!(f, "Shot Package"),
        }
    }
}
