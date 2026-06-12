use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy)]
pub enum ArcGrenade {
    LightningGrenade,
    StormGrenade,
    FlashbangGrenade,
    PulseGrenade,
    SkipGrenade,
    FluxGrenade,
    ArcboltGrenade,
}

impl Display for ArcGrenade {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::LightningGrenade => "Lightning Grenade",
            Self::StormGrenade => "Storm Grenade",
            Self::FlashbangGrenade => "Flashbang Grenade",
            Self::PulseGrenade => "Pulse Grenade",
            Self::SkipGrenade => "Skip Grenade",
            Self::FluxGrenade => "Flux Grenade",
            Self::ArcboltGrenade => "Arcbolt Grenade",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum VoidGrenade {
    VoidSpike,
    VoidWall,
    SuppressorGrenade,
    VortexGrenade,
    ScatterGrenade,
    MagneticGrenade,
    AxionBolt,
}

impl Display for VoidGrenade {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::VoidSpike => "Void Spike",
            Self::VoidWall => "Void Wall",
            Self::SuppressorGrenade => "Suppressor Grenade",
            Self::VortexGrenade => "Vortex Grenade",
            Self::ScatterGrenade => "Scatter Grenade",
            Self::MagneticGrenade => "Magnetic Grenade",
            Self::AxionBolt => "Axion Bolt",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum SolarGrenade {
    TripmineGrenade,
    ThermiteGrenade,
    IncendiaryGrenade,
    SolarGrenade,
    SwarmGrenade,
    FusionGrenade,
    FireboltGrenade,
    HealingGrenade,
}

impl Display for SolarGrenade {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::TripmineGrenade => "tripmine_grenade",
            Self::ThermiteGrenade => "thermite_grenade",
            Self::IncendiaryGrenade => "incendiary_grenade",
            Self::SolarGrenade => "solar_grenade",
            Self::SwarmGrenade => "swarm_grenade",
            Self::FusionGrenade => "fusion_grenade",
            Self::FireboltGrenade => "firebolt_grenade",
            Self::HealingGrenade => "healing_grenade",
        };

        write!(f, "{s}")
    }
}
