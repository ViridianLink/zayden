use super::weapons::PHONEUTRIA_FERA;
use super::{
    Abilities,
    Armour,
    ArmourName,
    Artifact,
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
    Subclass,
    Super,
    Tag,
};

pub(super) const PRISMATIC_TITAN: Loadout<'_> = Loadout {
    name: "Insurmountable Skullfort",
    class: DestinyClass::Titan,
    mode: Mode::PvE,
    tags: [Some(Tag::AbilityFocused), None, None],
    subclass: SUBCLASS,
    gear: GEAR,
    artifact: Artifact::Unknown([
        Some(ArtifactPerk::RefreshThreads),
        Some(ArtifactPerk::ElementalCoalescence),
        Some(ArtifactPerk::Shieldcrush),
        Some(ArtifactPerk::RadiantShrapnel),
        None,
        None,
        None,
        None,
    ]),
    details: Details::new("LlamaD2", "https://dim.gg/iirdyoy/Prismatic")
        .video("https://youtu.be/STuEYFaGs84"),
};

const SUBCLASS: Subclass = Subclass {
    kind: Subclass::Prismatic,
    abilities: ABILITIES,
    aspects: [Aspect::Knockout, Aspect::DiamondLance],
    fragments: [
        Some(Fragment::FacetOfDawn),
        Some(Fragment::FacetOfProtection),
        Some(Fragment::FacetOfCourage),
        Some(Fragment::FacetOfAwakening),
        Some(Fragment::FacetOfSacrifice),
    ],
};

const ABILITIES: Abilities = Abilities {
    super_: Super::Thundercrash,
    class: ClassAbility::Thruster,
    jump: Jump::Catapult,
    melee: Melee::Thunderclap,
    grenade: Grenade::Shackle,
};

const GEAR: Gear<'_> = Gear {
    weapons: [None, Some(PHONEUTRIA_FERA), None],
    armour: [
        Armour::new(ArmourName::AnInsurmountableSkullfort, [
            Mod::HandsOn,
            Mod::HandsOn,
            Mod::Empty,
        ]),
        Armour::new(ArmourName::CollectivePsycheGauntlets, [
            Mod::HeavyHanded,
            Mod::MeleeFont,
            Mod::MeleeFont,
        ]),
        Armour::new(ArmourName::CollectivePsychePlate, [Mod::Empty; 3]),
        Armour::new(ArmourName::CollectivePsycheGreaves, [
            Mod::Innervation,
            Mod::StacksOnStacks,
            Mod::Empty,
        ]),
        Armour::new(ArmourName::CollectivePsycheMark, [
            Mod::TimeDilation,
            Mod::PowerfulAttraction,
            Mod::PowerfulAttraction,
        ]),
    ],
    stats_priority: [
        Stat::Melee(200),
        Stat::Super(200),
        Stat::Grenade(200),
        Stat::Weapons(200),
        Stat::Health(200),
        Stat::Class(200),
    ],
};
