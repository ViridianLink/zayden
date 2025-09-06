use super::weapons::PHONEUTRIA_FERA;
use super::{
    Abilities, Armour, ArmourName, ArtifactPerk, Aspect, ClassAbility, DestinyClass, Details,
    Fragment, Gear, Grenade, Jump, Loadout, Melee, Mod, Mode, Stat, Subclass, SubclassType, Super,
    Tag,
};

pub const PRISMATIC_TITAN: Loadout = Loadout::new(
    "Insurmountable Skullfort",
    DestinyClass::Titan,
    Mode::PvE,
    SUBCLASS,
    GEAR,
    Details::new("LlamaD2", "https://dim.gg/iirdyoy/Prismatic")
        .video("https://youtu.be/STuEYFaGs84"),
)
.tags([Some(Tag::AbilityFocused), None, None])
.artifact([
    Some(ArtifactPerk::RefreshThreads),
    Some(ArtifactPerk::ElementalCoalescence),
    Some(ArtifactPerk::Shieldcrush),
    Some(ArtifactPerk::RadiantShrapnel),
    None,
    None,
    None,
]);

const SUBCLASS: Subclass = Subclass {
    subclass: SubclassType::Prismatic,
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
    jump: Jump::CatapultLift,
    melee: Melee::Thunderclap,
    grenade: Grenade::Shackle,
};

const GEAR: Gear = Gear {
    weapons: [None, Some(PHONEUTRIA_FERA), None],
    armour: [
        Armour::new(
            ArmourName::AnInsurmountableSkullfort,
            [Mod::HandsOn, Mod::HandsOn, Mod::Empty],
        ),
        Armour::new(
            ArmourName::CollectivePsycheGauntlets,
            [Mod::HeavyHanded, Mod::MeleeFont, Mod::MeleeFont],
        ),
        Armour::new(ArmourName::CollectivePsychePlate, [Mod::Empty; 3]),
        Armour::new(
            ArmourName::CollectivePsycheGreaves,
            [Mod::Innervation, Mod::StacksOnStacks, Mod::Empty],
        ),
        Armour::new(
            ArmourName::CollectivePsycheMark,
            [
                Mod::TimeDilation,
                Mod::PowerfulAttraction,
                Mod::PowerfulAttraction,
            ],
        ),
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
