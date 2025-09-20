use super::weapons::DEAD_MESSENGER;
use super::{
    Abilities, Armour, ArmourName, ArtifactPerk, Aspect, ClassAbility, DestinyClass, Details,
    Fragment, Gear, Grenade, Jump, Loadout, Melee, Mod, Mode, Stat, Subclass, SubclassType, Super,
};

pub const VOID_WARLOCK: Loadout = Loadout {
    name: "Handheld Supernova",
    class: DestinyClass::Warlock,
    mode: Mode::PvE,
    tags: [None; 3],
    subclass: SUBCLASS,
    gear: GEAR,
    artifact: [
        Some(ArtifactPerk::RefreshThreads),
        Some(ArtifactPerk::ElementalCoalescence),
        Some(ArtifactPerk::Shieldcrush),
        None,
        None,
        None,
        None,
        None,
    ],
    details: Details::new("LlamaD2", "https://dim.gg/fiauzci/Void")
        .video("https://youtu.be/TBbOiMWPIkE"),
};

const SUBCLASS: Subclass = Subclass {
    subclass: SubclassType::Void,
    abilities: ABILITIES,
    aspects: [Aspect::ChaosAccelerant, Aspect::FeedTheVoid],
    fragments: [
        Some(Fragment::EchoOfPersistence),
        Some(Fragment::EchoOfInstability),
        Some(Fragment::EchoOfExpulsion),
        Some(Fragment::EchoOfVigilance),
        None,
    ],
};

const ABILITIES: Abilities = Abilities {
    super_: Super::NovaBombCataclysm,
    class: ClassAbility::HealingRift,
    jump: Jump::BurstGlide,
    melee: Melee::PocketSingularity,
    grenade: Grenade::Magnetic,
};

const GEAR: Gear = Gear {
    weapons: [None, Some(DEAD_MESSENGER), None],
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
            ArmourName::AionAdapterGloves,
            [Mod::GrenadeFont, Mod::GrenadeFont, Mod::Firepower],
        ),
        Armour::new(
            ArmourName::AionAdapterRobes,
            [Mod::VoidAmmoGeneration, Mod::Empty, Mod::Empty],
        ),
        Armour::new(
            ArmourName::AionAdapterBoots,
            [
                Mod::StacksOnStacks,
                Mod::WeaponsFont,
                Mod::HarmonicScavenger,
            ],
        ),
        Armour::new(
            ArmourName::AionAdapterBond,
            [Mod::TimeDilation, Mod::Reaper, Mod::SpecialFinisher],
        ),
    ],
    stats_priority: [
        Stat::Grenade(200),
        Stat::Super(200),
        Stat::Weapons(200),
        Stat::Melee(200),
        Stat::Class(200),
        Stat::Health(200),
    ],
};
