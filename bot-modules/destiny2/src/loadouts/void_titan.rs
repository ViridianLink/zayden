use super::weapons::DEAD_MESSENGER;
use super::{
    Abilities,
    Armour,
    ArmourName,
    ArtifactPerk,
    Aspect,
    ClassAbility,
    DestinyClass,
    Details,
    Fragment,
    Gear,
    Grenade,
    Jump,
    Loadout,
    Melee,
    Mod,
    Mode,
    Stat,
    Subclass,
    SubclassType,
    Super,
};

pub(super) const VOID_TITAN: Loadout<'_> = Loadout {
    name: "Shield Bash",
    class: DestinyClass::Titan,
    mode: Mode::PvE,
    tags: [None; 3],
    subclass: SUBCLASS,
    gear: GEAR,
    artifact: 
    artifact_perks: [
        Some(ArtifactPerk::Shieldcrush),
        Some(ArtifactPerk::ExpandingAbyss),
        Some(ArtifactPerk::VoidHegemony),
        None,
        None,
        None,
        None,
        None,
    ],
    details: Details::new("LlamaD2", "https://dim.gg/3amqc4y/Void")
        .video("https://youtu.be/IhBfmN00LEs"),
};

const SUBCLASS: Subclass = Subclass {
    kind: SubclassType::Void,
    abilities: ABILITIES,
    aspects: [Aspect::Bastion, Aspect::OffensiveBulwark],
    fragments: [
        Some(Fragment::EchoOfStarvation),
        Some(Fragment::EchoOfUndermining),
        Some(Fragment::EchoOfPersistence),
        Some(Fragment::EchoOfProvision),
        Some(Fragment::EchoOfLeeching),
    ],
};

const ABILITIES: Abilities = Abilities {
    super_: Super::TwilightArsenal,
    class: ClassAbility::RallyBarricade,
    jump: Jump::CatapultLift,
    melee: Melee::ShieldBash,
    grenade: Grenade::Vortex,
};

const GEAR: Gear<'_> = Gear {
    weapons: [None, Some(LOTUS_EATER), None],
    armour: [
        Armour::new(ArmourName::TitanHelm, [
            Mod::HandsOn,
            Mod::HandsOn,
            Mod::HarmonicSiphon,
        ]),
        Armour::new(ArmourName::TitanGauntlets, [
            Mod::HeavyHanded,
            Mod::HeavyHanded,
            Mod::HeavyHanded,
        ]),
        Armour::new(ArmourName::TitanPlate, [
            Mod::ConcussiveDampener,
            Mod::Empty,
            Mod::Empty,
        ]),
        Armour::new(ArmourName::PeregrineGreaves, [
            Mod::StacksOnStacks,
            Mod::Absolution,
            Mod::Invigoration,
        ]),
        Armour::new(ArmourName::TitanMark, [
            Mod::PowerfulAttraction,
            Mod::ClassFont,
            Mod::ClassFont,
        ]),
    ],
    stats_priority: [
        Stat::Melee(200),
        Stat::Super(200),
        Stat::Health(200),
        Stat::Grenade(200),
        Stat::Class(200),
        Stat::Weapons(200),
    ],
};
