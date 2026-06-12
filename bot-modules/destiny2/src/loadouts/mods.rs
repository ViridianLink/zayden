use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Clone, Copy)]
pub enum HelmetMod {
    Empty,
    SpecialAmmoFinder,
    SpecialAmmoScout,
    HeavyAmmoFinder,
    HeavyAmmoScout,
    Dynamo,
    AshesToAssets,
    HandsOn,
    PowerPreservation,
    RadiantLight,
    PowerfulFriends,
    SuperFont,
    HarmonicSiphon,
    KineticSiphon,
    ArcSiphon,
    SolarSiphon,
    StasisSiphon,
    StrandSiphon,
    VoidSiphon,
    InFlightCompensator,
    HarmonicTargeting,
    KineticTargeting,
    ArcTargeting,
    SolarTargeting,
    StasisTargeting,
    StrandTargeting,
    VoidTargeting,
}

impl Display for HelmetMod {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Empty => "empty_mod",
            Self::SpecialAmmoFinder => "special_ammo_finder",
            Self::SpecialAmmoScout => "special_ammo_scout",
            Self::HeavyAmmoFinder => "heavy_ammo_finder",
            Self::HeavyAmmoScout => "heavy_ammo_scout",
            Self::Dynamo => "dynamo",
            Self::AshesToAssets => "ashes_to_assets",
            Self::HandsOn => "hands_on",
            Self::PowerPreservation => "power_preservation",
            Self::RadiantLight => "radiant_light",
            Self::PowerfulFriends => "powerful_friends",
            Self::SuperFont => "super_font",
            Self::HarmonicSiphon => "harmonic_siphon",
            Self::KineticSiphon => "kinetic_siphon",
            Self::ArcSiphon => "arc_siphon",
            Self::SolarSiphon => "solar_siphon",
            Self::StasisSiphon => "stasis_siphon",
            Self::StrandSiphon => "strand_siphon",
            Self::VoidSiphon => "void_siphon",
            Self::InFlightCompensator => "in_flight_compensator",
            Self::HarmonicTargeting => "harmonic_targeting",
            Self::KineticTargeting => "kinetic_targeting",
            Self::ArcTargeting => "arc_targeting",
            Self::SolarTargeting => "solar_targeting",
            Self::StasisTargeting => "stasis_targeting",
            Self::StrandTargeting => "strand_targeting",
            Self::VoidTargeting => "void_targeting",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum ArmsMod {
    Empty,
    Fastball,
    Firepower,
    ImpactInduction,
    BolsteringDetonation,
    GrenadeKickstart,
    GrenadeFont,
    HeavyHanded,
    MomentumTransfer,
    FocusingStrike,
    MeleeKickstart,
    MeleeFont,
    ShieldBreakCharge,
    HarmonicLoader,
    KineticLoader,
    ArcLoader,
    SolarLoader,
    StasisLoader,
    StrandLoader,
    VoidLoader,
    HarmonicDexterity,
    KineticDexterity,
    ArcDexterity,
    SolarDexterity,
    StasisDexterity,
    StrandDexterity,
    VoidDexterity,
}

impl Display for ArmsMod {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Empty => "empty_mod",
            Self::Fastball => "fastball",
            Self::Firepower => "firepower",
            Self::ImpactInduction => "impact_induction",
            Self::BolsteringDetonation => "bolstering_detonation",
            Self::GrenadeKickstart => "grenade_kickstart",
            Self::GrenadeFont => "grenade_font",
            Self::HeavyHanded => "heavy_handed",
            Self::MomentumTransfer => "momentum_transfer",
            Self::FocusingStrike => "focusing_strike",
            Self::MeleeKickstart => "melee_kickstart",
            Self::MeleeFont => "melee_font",
            Self::ShieldBreakCharge => "shield_break_charge",
            Self::HarmonicLoader => "harmonic_loader",
            Self::KineticLoader => "kinetic_loader",
            Self::ArcLoader => "arc_loader",
            Self::SolarLoader => "solar_loader",
            Self::StasisLoader => "stasis_loader",
            Self::StrandLoader => "strand_loader",
            Self::VoidLoader => "void_loader",
            Self::HarmonicDexterity => "harmonic_dexterity",
            Self::KineticDexterity => "kinetic_dexterity",
            Self::ArcDexterity => "arc_dexterity",
            Self::SolarDexterity => "solar_dexterity",
            Self::StasisDexterity => "stasis_dexterity",
            Self::StrandDexterity => "strand_dexterity",
            Self::VoidDexterity => "void_dexterity",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum ChestMod {
    Empty,
    HarmonicResistance,
    ArcResistance,
    SolarResistance,
    VoidResistance,
    StasisResistance,
    StrandResistance,
    ConcussiveDampener,
    MeleeDamageResistance,
    SniperDamageResistance,
    EmergencyReinforcement,
    HealthFont,
    ChargedUp,
    LucentBlades,
    UnflinchingHarmonicAim,
    UnflinchingKineticAim,
    UnflinchingArcAim,
    UnflinchingSolarAim,
    UnflinchingStasisAim,
    UnflinchingStrandAim,
    UnflinchingVoidAim,
    HarmonicAmmoGeneration,
    KineticAmmoGeneration,
    ArcAmmoGeneration,
    SolarAmmoGeneration,
    StasisAmmoGeneration,
    StrandAmmoGeneration,
    VoidAmmoGeneration,
}

impl Display for ChestMod {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Empty => "empty_mod",
            Self::HarmonicResistance => "harmonic_resistance",
            Self::ArcResistance => "arc_resistance",
            Self::SolarResistance => "solar_resistance",
            Self::VoidResistance => "void_resistance",
            Self::StasisResistance => "stasis_resistance",
            Self::StrandResistance => "strand_resistance",
            Self::ConcussiveDampener => "concussive_dampener",
            Self::MeleeDamageResistance => "melee_damage_resistance",
            Self::SniperDamageResistance => "sniper_damage_resistance",
            Self::EmergencyReinforcement => "emergency_reinforcement",
            Self::HealthFont => "health_font",
            Self::ChargedUp => "charged_up",
            Self::LucentBlades => "lucent_blades",
            Self::UnflinchingHarmonicAim => "unflinching_harmonic_aim",
            Self::UnflinchingKineticAim => "unflinching_kinetic_aim",
            Self::UnflinchingArcAim => "unflinching_arc_aim",
            Self::UnflinchingSolarAim => "unflinching_solar_aim",
            Self::UnflinchingStasisAim => "unflinching_stasis_aim",
            Self::UnflinchingStrandAim => "unflinching_strand_aim",
            Self::UnflinchingVoidAim => "unflinching_void_aim",
            Self::HarmonicAmmoGeneration => "harmonic_ammo_generation",
            Self::KineticAmmoGeneration => "kinetic_ammo_generation",
            Self::ArcAmmoGeneration => "arc_ammo_generation",
            Self::SolarAmmoGeneration => "solar_ammo_generation",
            Self::StasisAmmoGeneration => "stasis_ammo_generation",
            Self::StrandAmmoGeneration => "strand_ammo_generation",
            Self::VoidAmmoGeneration => "void_ammo_generation",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum LegsMod {
    Empty,
    Recuperation,
    BetterAlready,
    Innervation,
    Invigoration,
    Insulation,
    Absolution,
    OrbsOfRestoration,
    EnhancedAthletics,
    StacksOnStacks,
    ElementalCharge,
    KineticWeaponSurge,
    ArcWeaponSurge,
    SolarWeaponSurge,
    StasisWeaponSurge,
    StrandWeaponSurge,
    VoidWeaponSurge,
    WeaponsFont,
    HarmonicHolster,
    KineticHolster,
    ArcHolster,
    SolarHolster,
    StasisHolster,
    StrandHolster,
    VoidHolster,
    HarmonicScavenger,
    KineticScavenger,
    ArcScavenger,
    SolarScavenger,
    StasisScavenger,
    StrandScavenger,
    VoidScavenger,
}

impl Display for LegsMod {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Empty => "empty_mod",
            Self::Recuperation => "recuperation",
            Self::BetterAlready => "better_already",
            Self::Innervation => "innervation",
            Self::Invigoration => "invigoration",
            Self::Insulation => "insulation",
            Self::Absolution => "absolution",
            Self::OrbsOfRestoration => "orbs_of_restoration",
            Self::EnhancedAthletics => "enhanced_athletics",
            Self::StacksOnStacks => "stacks_on_stacks",
            Self::ElementalCharge => "elemental_charge",
            Self::KineticWeaponSurge => "kinetic_weapon_surge",
            Self::ArcWeaponSurge => "arc_weapon_surge",
            Self::SolarWeaponSurge => "solar_weapon_surge",
            Self::StasisWeaponSurge => "stasis_weapon_surge",
            Self::StrandWeaponSurge => "strand_weapon_surge",
            Self::VoidWeaponSurge => "void_weapon_surge",
            Self::WeaponsFont => "weapons_font",
            Self::HarmonicHolster => "harmonic_holster",
            Self::KineticHolster => "kinetic_holster",
            Self::ArcHolster => "arc_holster",
            Self::SolarHolster => "solar_holster",
            Self::StasisHolster => "stasis_holster",
            Self::StrandHolster => "strand_holster",
            Self::VoidHolster => "void_holster",
            Self::HarmonicScavenger => "harmonic_scavenger",
            Self::KineticScavenger => "kinetic_scavenger",
            Self::ArcScavenger => "arc_scavenger",
            Self::SolarScavenger => "solar_scavenger",
            Self::StasisScavenger => "stasis_scavenger",
            Self::StrandScavenger => "strand_scavenger",
            Self::VoidScavenger => "void_scavenger",
        };

        write!(f, "{s}")
    }
}

#[derive(Clone, Copy)]
pub enum ClassItemMod {
    Empty,
    Distribution,
    Outreach,
    Bomber,
    UtilityKickstart,
    ClassFont,
    TimeDilation,
    PowerfulAttraction,
    ProximityWard,
    RestorativeFinisher,
    SpecialFinisher,
    OneTwoFinisher,
    BulwarkFinisher,
    HealthyFinisher,
    SnaploadFinisher,
    ExplosiveFinisher,
    UtilityFinisher,
    BenevolentFinisher,
    EmpoweredFinish,
    Reaper,
}

impl Display for ClassItemMod {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Empty => "empty_mod",
            Self::Distribution => "distribution",
            Self::Outreach => "outreach",
            Self::Bomber => "bomber",
            Self::UtilityKickstart => "utility_kickstart",
            Self::ClassFont => "class_font",
            Self::TimeDilation => "time_dilation",
            Self::PowerfulAttraction => "powerful_attraction",
            Self::ProximityWard => "proximity_ward",
            Self::RestorativeFinisher => "restorative_finisher",
            Self::SpecialFinisher => "special_finisher",
            Self::OneTwoFinisher => "one_two_finisher",
            Self::BulwarkFinisher => "bulwark_finisher",
            Self::HealthyFinisher => "healthy_finisher",
            Self::SnaploadFinisher => "snapload_finisher",
            Self::ExplosiveFinisher => "explosive_finisher",
            Self::UtilityFinisher => "utility_finisher",
            Self::BenevolentFinisher => "benevolent_finisher",
            Self::EmpoweredFinish => "empowered_finish",
            Self::Reaper => "reaper",
        };

        write!(f, "{s}")
    }
}
