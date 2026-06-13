use std::fmt;
use std::fmt::{Display, Formatter};

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum ArcGrenade {
    Lightning,
    Storm,
    Flashbang,
    Pulse,
    Skip,
    Flux,
    Arcbolt,
}

impl Display for ArcGrenade {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Lightning => "lightning_grenade",
            Self::Storm => "storm_grenade",
            Self::Flashbang => "flashbang_grenade",
            Self::Pulse => "pulse_grenade",
            Self::Skip => "skip_grenade",
            Self::Flux => "flux_grenade",
            Self::Arcbolt => "arcbolt_grenade",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum SolarGrenade {
    Tripmine,
    Thermite,
    Incendiary,
    Solar,
    Swarm,
    Fusion,
    Firebolt,
    Healing,
}

impl Display for SolarGrenade {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Tripmine => "tripmine_grenade",
            Self::Thermite => "thermite_grenade",
            Self::Incendiary => "incendiary_grenade",
            Self::Solar => "solar_grenade",
            Self::Swarm => "swarm_grenade",
            Self::Fusion => "fusion_grenade",
            Self::Firebolt => "firebolt_grenade",
            Self::Healing => "healing_grenade",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum VoidGrenade {
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
            Self::VoidSpike => "void_spike",
            Self::VoidWall => "void_wall",
            Self::SuppressorGrenade => "suppressor_grenade",
            Self::VortexGrenade => "vortex_grenade",
            Self::ScatterGrenade => "scatter_grenade",
            Self::MagneticGrenade => "magnetic_grenade",
            Self::AxionBolt => "axion_bolt",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum StasisGrenade {
    Glacier,
    Duskfield,
    Coldsnap,
    Shatter,
}

impl Display for StasisGrenade {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Glacier => "glacier_grenade",
            Self::Duskfield => "duskfield_grenade",
            Self::Coldsnap => "coldsnap_grenade",
            Self::Shatter => "shatter_grenade",
        };

        write!(f, "{s}")
    }
}

#[expect(dead_code, reason = "reserved for future loadout builds")]
#[derive(Clone, Copy)]
pub(super) enum StrandGrenade {
    SlicewireGrenade,
    ShackleGrenade,
    ThreadlingGrenade,
    Grapple,
}

impl Display for StrandGrenade {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::SlicewireGrenade => "slicewire_grenade",
            Self::ShackleGrenade => "shackle_grenade",
            Self::ThreadlingGrenade => "threadling_grenade",
            Self::Grapple => "grapple",
        };

        write!(f, "{s}")
    }
}
