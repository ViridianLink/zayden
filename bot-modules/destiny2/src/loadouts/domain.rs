use std::fmt;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(type_name = "destiny2_class", rename_all = "lowercase")]
pub enum Class {
    Hunter,
    Titan,
    Warlock,
}

impl Class {
    pub const ALL: [Self; 3] = [Self::Hunter, Self::Titan, Self::Warlock];
}

impl Display for Class {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Hunter => "Hunter",
            Self::Titan => "Titan",
            Self::Warlock => "Warlock",
        };
        write!(f, "{s}")
    }
}

impl FromStr for Class {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Hunter" | "hunter" => Ok(Self::Hunter),
            "Titan" | "titan" => Ok(Self::Titan),
            "Warlock" | "warlock" => Ok(Self::Warlock),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(type_name = "destiny2_element", rename_all = "lowercase")]
pub enum Element {
    Arc,
    Solar,
    Void,
    Strand,
    Stasis,
    Prismatic,
}

impl Element {
    pub const ALL: [Self; 6] = [
        Self::Arc,
        Self::Solar,
        Self::Void,
        Self::Strand,
        Self::Stasis,
        Self::Prismatic,
    ];

    #[must_use]
    pub fn key(self) -> String {
        self.to_string().to_lowercase()
    }
}

impl Display for Element {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Arc => "Arc",
            Self::Solar => "Solar",
            Self::Void => "Void",
            Self::Strand => "Strand",
            Self::Stasis => "Stasis",
            Self::Prismatic => "Prismatic",
        };
        write!(f, "{s}")
    }
}

impl FromStr for Element {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Arc" => Ok(Self::Arc),
            "Solar" => Ok(Self::Solar),
            "Void" => Ok(Self::Void),
            "Strand" => Ok(Self::Strand),
            "Stasis" => Ok(Self::Stasis),
            "Prismatic" => Ok(Self::Prismatic),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(type_name = "destiny2_armour_slot", rename_all = "snake_case")]
pub enum ArmourSlot {
    Helmet,
    Arms,
    Chest,
    Legs,
    ClassItem,
}

impl ArmourSlot {
    pub const ALL: [Self; 5] =
        [Self::Helmet, Self::Arms, Self::Chest, Self::Legs, Self::ClassItem];

    #[must_use]
    pub const fn render_order() -> [Self; 5] {
        Self::ALL
    }
}

impl Display for ArmourSlot {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Helmet => "Helmet",
            Self::Arms => "Arms",
            Self::Chest => "Chest",
            Self::Legs => "Legs",
            Self::ClassItem => "Class Item",
        };
        write!(f, "{s}")
    }
}

impl FromStr for ArmourSlot {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL.into_iter().find(|a| a.to_string() == s).ok_or(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(type_name = "destiny2_stat", rename_all = "lowercase")]
pub enum StatKind {
    Health,
    Melee,
    Grenade,
    Super,
    Class,
    Weapons,
}

impl StatKind {
    pub const ALL: [Self; 6] = [
        Self::Health,
        Self::Melee,
        Self::Grenade,
        Self::Super,
        Self::Class,
        Self::Weapons,
    ];
}

impl Display for StatKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Health => "health",
            Self::Melee => "melee",
            Self::Grenade => "grenade",
            Self::Super => "super",
            Self::Class => "class",
            Self::Weapons => "weapons",
        };
        write!(f, "{s}")
    }
}

impl FromStr for StatKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL.into_iter().find(|k| k.to_string() == s).ok_or(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "destiny2_archetype", rename_all = "snake_case")]
pub enum Archetype {
    AutoRifle,
    Bow,
    FusionRifle,
    Glaive,
    BreechGrenadeLauncher,
    GrenadeLauncher,
    HandCannon,
    LinearFusionRifle,
    MachineGun,
    RocketPulseRifle,
    PulseRifle,
    RocketLauncher,
    ScoutRifle,
    Shotgun,
    RocketSidearm,
    Sidearm,
    Smg,
    SniperRifle,
    Sword,
    TraceRifle,
}

impl Archetype {
    pub const ALL: [Self; 20] = [
        Self::AutoRifle,
        Self::Bow,
        Self::FusionRifle,
        Self::Glaive,
        Self::BreechGrenadeLauncher,
        Self::GrenadeLauncher,
        Self::HandCannon,
        Self::LinearFusionRifle,
        Self::MachineGun,
        Self::RocketPulseRifle,
        Self::PulseRifle,
        Self::RocketLauncher,
        Self::ScoutRifle,
        Self::Shotgun,
        Self::RocketSidearm,
        Self::Sidearm,
        Self::Smg,
        Self::SniperRifle,
        Self::Sword,
        Self::TraceRifle,
    ];
}

impl Display for Archetype {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::AutoRifle => "Auto Rifle",
            Self::Bow => "Bow",
            Self::FusionRifle => "Fusion Rifle",
            Self::Glaive => "Glaive",
            Self::BreechGrenadeLauncher => "Breech Grenade Launcher",
            Self::GrenadeLauncher => "Grenade Launcher",
            Self::HandCannon => "Hand Cannon",
            Self::LinearFusionRifle => "Linear Fusion Rifle",
            Self::MachineGun => "Machine Gun",
            Self::RocketPulseRifle => "Rocket Pulse Rifle",
            Self::PulseRifle => "Pulse Rifle",
            Self::RocketLauncher => "Rocket Launcher",
            Self::ScoutRifle => "Scout Rifle",
            Self::Shotgun => "Shotgun",
            Self::RocketSidearm => "Rocket Sidearm",
            Self::Sidearm => "Sidearm",
            Self::Smg => "SMG",
            Self::SniperRifle => "Sniper Rifle",
            Self::Sword => "Sword",
            Self::TraceRifle => "Trace Rifle",
        };
        write!(f, "{s}")
    }
}

impl FromStr for Archetype {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::ALL.into_iter().find(|a| a.to_string() == s).ok_or(())
    }
}
