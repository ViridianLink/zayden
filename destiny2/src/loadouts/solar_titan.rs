use super::weapons::{DEVILS_RUIN, PERFECT_PARADOX};
use super::{
    Abilities, Armour, ArmourName, ArtifactPerk, Aspect, ClassAbility, DestinyClass, Details,
    Fragment, Gear, Grenade, Jump, Loadout, Melee, Mod, Mode, Stat, Subclass, SubclassType, Super,
    Tag,
};

pub const SOLAR_TITAN: Loadout = Loadout {
    name: "Throwing Hammer",
    class: DestinyClass::Titan,
    mode: Mode::PvE,
    tags: [Some(Tag::HighSurvivability), None, None],
    subclass: SUBCLASS,
    gear: GEAR,
    artifact: [
        Some(ArtifactPerk::DivinersDiscount),
        Some(ArtifactPerk::ReciprocalDraw),
        Some(ArtifactPerk::RefreshThreads),
        Some(ArtifactPerk::ElementalCoalescence),
        Some(ArtifactPerk::RadiantShrapnel),
        Some(ArtifactPerk::ElementalOverdrive),
        None,
    ],
    details: Details {
        author: "Bradley",
        dim_link: "https://dim.gg/dundpfq/Throwing-Hammer",
        how_it_works: None,
        video: Some("https://youtu.be/17M4VhtDNqc"),
    },
};

const SUBCLASS: Subclass = Subclass {
    subclass: SubclassType::Solar,
    abilities: ABILITIES,
    aspects: [Aspect::RoaringFlames, Aspect::SolInvictus],
    fragments: [
        Some(Fragment::EmberOfSearing),
        Some(Fragment::EmberOfTorches),
        Some(Fragment::EmberOfEmpyrean),
        Some(Fragment::EmberOfAshes),
        None,
    ],
};

const ABILITIES: Abilities = Abilities {
    super_: Super::BurningMaul,
    class: ClassAbility::RallyBarricade,
    jump: Jump::CatapultLift,
    melee: Melee::ThrowingHammer,
    grenade: Grenade::Fusion,
};

const GEAR: Gear = Gear {
    weapons: [Some(PERFECT_PARADOX), Some(DEVILS_RUIN), None],
    armour: [
        Armour::new(
            ArmourName::LustrousHelm,
            [Mod::HandsOn, Mod::SpecialAmmoFinder, Mod::HarmonicSiphon],
        ),
        Armour::new(
            ArmourName::WormgodCaress,
            [Mod::MeleeFont, Mod::MeleeFont, Mod::HeavyHanded],
        ),
        Armour::new(ArmourName::LustrousPlate, [Mod::Empty; 3]),
        Armour::new(
            ArmourName::LustrousGreaves,
            [Mod::StacksOnStacks, Mod::KineticScavenger, Mod::Empty],
        ),
        Armour::new(
            ArmourName::LustrousMark,
            [Mod::TimeDilation, Mod::Distribution, Mod::SpecialFinisher],
        ),
    ],
    stats_priority: [
        Stat::Melee,
        Stat::Super,
        Stat::Grenade,
        Stat::Health,
        Stat::Weapons,
        Stat::Class,
    ],
};
