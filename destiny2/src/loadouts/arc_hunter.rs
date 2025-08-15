use super::weapons::GRAVITON_SPIKE;
use super::{
    Abilities, Armour, ArtifactPerk, Aspect, ClassAbility, DestinyClass, Details, Fragment, Gear,
    Grenade, Jump, Loadout, Melee, Mod, Mode, Stat, Subclass, SubclassType, Super,
};

pub const ARC_HUNTER: Loadout = Loadout::new(
    "Mask of Bakris",
    DestinyClass::Hunter,
    Mode::PvE,
    SUBCLASS,
    GEAR,
    Details::new("LlamaD2", "https://dim.gg/kdkiusy/Arc").video("https://youtu.be/FKY7N2cb5Zc"),
)
.artifact([
    Some(ArtifactPerk::OneWithFrost),
    Some(ArtifactPerk::FeverAndChill),
    Some(ArtifactPerk::FrostRenewal),
    Some(ArtifactPerk::RefreshThreads),
    Some(ArtifactPerk::Shieldcrush),
    Some(ArtifactPerk::FrigidGlare),
    None,
]);

const SUBCLASS: Subclass = Subclass {
    subclass: SubclassType::Arc,
    abilities: ABILITIES,
    aspects: [Aspect::TempestStrike, Aspect::FlowState],
    fragments: [
        Some(Fragment::SparkOfResistance),
        Some(Fragment::SparkOfAmplitude),
        Some(Fragment::SparkOfFrequency),
        Some(Fragment::SparkOfDischarge),
        None,
    ],
};

const ABILITIES: Abilities = Abilities {
    super_: Super::GatheringStorm,
    class: ClassAbility::GamblersDodge,
    jump: Jump::Triple,
    melee: Melee::CombinationBlow,
    grenade: Grenade::Flux,
};

const GEAR: Gear = Gear {
    weapons: [None, Some(GRAVITON_SPIKE), None],
    armour: [
        Armour::new(
            "Mask of Bakris",
            [Mod::HarmonicSiphon, Mod::StasisSiphon, Mod::SuperFont],
        ),
        Armour::new(
            "Bushido Grips",
            [Mod::HarmonicLoader, Mod::ImpactInduction, Mod::HeavyHanded],
        ),
        Armour::new("Bushido Vest", [Mod::Empty; 3]),
        Armour::new(
            "Last Discipline Strides",
            [
                Mod::Recuperation,
                Mod::ArcWeaponSurge,
                Mod::StasisWeaponSurge,
            ],
        ),
        Armour::new(
            "Last Discipline Cloak",
            [Mod::TimeDilation, Mod::PowerfulAttraction, Mod::Reaper],
        ),
    ],
    stats_priority: [
        Stat::Class,
        Stat::Super,
        Stat::Melee,
        Stat::Grenade,
        Stat::Health,
        Stat::Weapons,
    ],
};
