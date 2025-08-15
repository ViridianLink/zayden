use super::weapons::MONTE_CARLO;
use super::{
    Abilities, Armour, ArtifactPerk, Aspect, ClassAbility, DestinyClass, Details, Fragment, Gear,
    Grenade, Jump, Loadout, Melee, Mod, Mode, Stat, Subclass, SubclassType, Super, Tag,
};

pub const STRAND_TITAN: Loadout = Loadout::new(
    "Flechette Storm",
    DestinyClass::Titan,
    Mode::PvE,
    SUBCLASS,
    GEAR,
    Details::new("LlamaD2", "https://dim.gg/g37nsna/Strand").video("https://youtu.be/T7KhZa1sBuA"),
)
.tags([Some(Tag::AbilityFocused), None, None])
.artifact([
    Some(ArtifactPerk::TightlyWoven),
    Some(ArtifactPerk::RefreshThreads),
    Some(ArtifactPerk::ThreadedBlast),
    Some(ArtifactPerk::Shieldcrush),
    Some(ArtifactPerk::TangledWeb),
    None,
    None,
]);

const SUBCLASS: Subclass = Subclass {
    subclass: SubclassType::Strand,
    abilities: ABILITIES,
    aspects: [Aspect::BannerOfWar, Aspect::FlechetteStorm],
    fragments: [
        Some(Fragment::ThreadOfFury),
        Some(Fragment::ThreadOfWarding),
        Some(Fragment::ThreadOfGeneration),
        Some(Fragment::ThreadOfTransmutation),
        None,
    ],
};

const ABILITIES: Abilities = Abilities {
    super_: Super::Bladefury,
    class: ClassAbility::RallyBarricade,
    jump: Jump::CatapultLift,
    melee: Melee::FrenziedBlade,
    grenade: Grenade::Grapple,
};

const GEAR: Gear = Gear {
    weapons: [Some(MONTE_CARLO), None, None],
    armour: [
        Armour::new(
            "Collective Psyche Helm",
            [Mod::KineticSiphon, Mod::HandsOn, Mod::HandsOn],
        ),
        Armour::new(
            "Wishful Ignorance",
            [Mod::HeavyHanded, Mod::MeleeFont, Mod::MeleeFont],
        ),
        Armour::new("Collective Psyche Plate", [Mod::Empty; 3]),
        Armour::new(
            "Collective Psyche Greaves",
            [Mod::Invigoration, Mod::Recuperation, Mod::StacksOnStacks],
        ),
        Armour::new(
            "Collective Psyche Mark",
            [Mod::TimeDilation, Mod::Outreach, Mod::Reaper],
        ),
    ],
    stats_priority: [
        Stat::Melee,
        Stat::Super,
        Stat::Grenade,
        Stat::Weapons,
        Stat::Health,
        Stat::Class,
    ],
};
