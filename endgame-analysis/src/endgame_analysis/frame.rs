use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

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
}

impl FromStr for Frame {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Rapid" => Ok(Frame::Rapid),
            "Rapid Slug" => Ok(Frame::RapidSlug),
            "Pinpoint Slug" => Ok(Frame::PinpointSlug),
            "Aggressive" => Ok(Frame::Aggressive),
            "Lightweight" => Ok(Frame::Lightweight),
            "Heavy Burst" => Ok(Frame::HeavyBurst),
            "Precision" => Ok(Frame::Precision),
            "Adaptive" => Ok(Frame::Adaptive),
            "High-Impact" => Ok(Frame::HighImpact),
            "Area Denial" => Ok(Frame::AreaDenial),
            "Micro-Missile" => Ok(Frame::MicroMissile),
            "Double Fire" => Ok(Frame::DoubleFire),
            "Wave" => Ok(Frame::Wave),
            "Compressed Wave" => Ok(Frame::CompressedWave),
            "Vortex" => Ok(Frame::Vortex),
            "Caster" => Ok(Frame::Caster),
            "Adaptive Burst" => Ok(Frame::AdaptiveBurst),
            "Support" => Ok(Frame::Support),
            "Aggressive Burst" => Ok(Frame::AggressiveBurst),
            "Legacy PR-55" => Ok(Frame::LegacyPR55),
            "Together Forever" => Ok(Frame::TogetherForever),
            "MIDA Synergy" => Ok(Frame::MIDASynergy),
            "High-Impact Longbow" => Ok(Frame::HighImpactLongBow),
            "Spread Shot" => Ok(Frame::SpreadShot),
            "Rocket-Assisted" => Ok(Frame::RocketAssisted),
            _ => {
                eprintln!("Failed to parse: '{s}'");
                Err(())
            }
        }
    }
}

impl fmt::Display for Frame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Frame::Rapid => write!(f, "Rapid"),
            Frame::RapidSlug => write!(f, "Rapid Slug"),
            Frame::PinpointSlug => write!(f, "Pinpoint Slug"),
            Frame::Aggressive => write!(f, "Aggressive"),
            Frame::Lightweight => write!(f, "Lightweight"),
            Frame::HeavyBurst => write!(f, "Heavy Burst"),
            Frame::Precision => write!(f, "Precision"),
            Frame::Adaptive => write!(f, "Adaptive"),
            Frame::HighImpact => write!(f, "High-Impact"),
            Frame::AreaDenial => write!(f, "Area Denial"),
            Frame::MicroMissile => write!(f, "Micro-Missile"),
            Frame::DoubleFire => write!(f, "Double Fire"),
            Frame::Wave => write!(f, "Wave"),
            Frame::CompressedWave => write!(f, "Compressed Wave"),
            Frame::Vortex => write!(f, "Vortex"),
            Frame::Caster => write!(f, "Caster"),
            Frame::AdaptiveBurst => write!(f, "Adaptive Burst"),
            Frame::Support => write!(f, "Support"),
            Frame::AggressiveBurst => write!(f, "Aggressive Burst"),
            Frame::LegacyPR55 => write!(f, "Legacy PR-55"),
            Frame::TogetherForever => write!(f, "Together Forever"),
            Frame::MIDASynergy => write!(f, "MIDA Synergy"),
            Frame::HighImpactLongBow => write!(f, "High-Impact Longbow"),
            Frame::SpreadShot => write!(f, "Spread Shot"),
            Frame::RocketAssisted => write!(f, "Rocket-Assisted"),
        }
    }
}
