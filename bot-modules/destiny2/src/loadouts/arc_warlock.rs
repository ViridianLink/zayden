use super::weapons::DELICATE_TOMB;
use super::{
    Abilities, Armour, ArmourName, ArtifactPerk, Aspect, ClassAbility, DestinyClass, Details,
    Fragment, Gear, Grenade, Jump, Loadout, Melee, Mod, Mode, Stat, Subclass, SubclassType, Super,
};

pub const ARC_WARLOCK: Loadout = Loadout::new(
    "Buddy Build",
    DestinyClass::Warlock,
    Mode::PvE,
    SUBCLASS,
    GEAR,
    Details::new("LlamaD2", "https://dim.gg/i2kny6a/Arc").video("https://youtu.be/sFzAdAl3ULw"),
)
.artifact([
    Some(ArtifactPerk::RefreshThreads),
    Some(ArtifactPerk::ElementalCoalescence),
    Some(ArtifactPerk::Shieldcrush),
    Some(ArtifactPerk::ElementalOverdrive),
    None,
    None,
    None,
    None,
]);

const SUBCLASS: Subclass = Subclass {
    subclass: SubclassType::Arc,
    abilities: ABILITIES,
    aspects: [Aspect::ArcSoul, Aspect::IonicSentry],
    fragments: [
        Some(Fragment::SparkOfShock),
        Some(Fragment::SparkOfResistance),
        Some(Fragment::SparkOfDischarge),
        Some(Fragment::SparkOfBeacons),
        None,
    ],
};

const ABILITIES: Abilities = Abilities {
    super_: Super::ChaosReach,
    class: ClassAbility::HealingRift,
    jump: Jump::BurstGlide,
    melee: Melee::BallLightning,
    grenade: Grenade::Pulse,
};

const GEAR: Gear = Gear {
    weapons: [None, Some(DELICATE_TOMB), None],
    armour: [
        Armour::new(
            ArmourName::VeritysBrow,
            [
                Mod::SpecialAmmoFinder,
                Mod::AshesToAssets,
                Mod::HarmonicSiphon,
            ],
        ),
        Armour::new(
            ArmourName::TechsecGloves,
            [Mod::GrenadeFont, Mod::BolsteringDetonation, Mod::Firepower],
        ),
        Armour::new(
            ArmourName::TechsecVestment,
            [Mod::HarmonicAmmoGeneration, Mod::Empty, Mod::Empty],
        ),
        Armour::new(
            ArmourName::TwofoldCrownBoots,
            [
                Mod::StacksOnStacks,
                Mod::WeaponsFont,
                Mod::HarmonicScavenger,
            ],
        ),
        Armour::new(
            ArmourName::TwofoldCrownBond,
            [Mod::TimeDilation, Mod::ClassFont, Mod::SpecialFinisher],
        ),
    ],
    stats_priority: [
        Stat::Grenade(200),
        Stat::Class(100),
        Stat::Super(200),
        Stat::Weapons(200),
        Stat::Melee(200),
        Stat::Health(200),
    ],
};
