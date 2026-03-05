use super::{
    Abilities, Armour, ArmourName, ArtifactPerk, Aspect, ClassAbility, DestinyClass, Details,
    Fragment, Gear, Grenade, Jump, Loadout, Melee, Mod, Mode, Stat, Subclass, SubclassType, Super,
    Tag,
};

pub const ARC_HUNTER: Loadout = Loadout::new(
    "Gifted Conviction",
    DestinyClass::Hunter,
    Mode::PvE,
    SUBCLASS,
    GEAR,
    Details::new("LlamaD2", "https://dim.gg/5e6byba/Arc").video("https://youtu.be/UDIJdVTl5SE"),
)
.tags([Some(Tag::EndGame), None, None])
.artifact([
    Some(ArtifactPerk::ElementalBenevolence),
    Some(ArtifactPerk::RefreshThreads),
    Some(ArtifactPerk::ElementalCoalescence),
    Some(ArtifactPerk::Shieldcrush),
    None,
    None,
    None,
    None,
]);

const SUBCLASS: Subclass = Subclass {
    subclass: SubclassType::Arc,
    abilities: ABILITIES,
    aspects: [Aspect::TempestStrike, Aspect::Ascension],
    fragments: [
        Some(Fragment::SparkOfResistance),
        Some(Fragment::SparkOfAmplitude),
        Some(Fragment::SparkOfFrequency),
        Some(Fragment::SparkOfIons),
        Some(Fragment::SparkOfFeedback),
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
    weapons: [None, None, None],
    armour: [
        Armour::new(
            ArmourName::HunterHelmet,
            [Mod::HandsOn, Mod::HandsOn, Mod::HarmonicSiphon],
        ),
        Armour::new(
            ArmourName::HunterArms,
            [Mod::MeleeFont, Mod::MeleeFont, Mod::HeavyHanded],
        ),
        Armour::new(ArmourName::GiftedConviction, [Mod::Empty; 3]),
        Armour::new(
            ArmourName::HunterLegs,
            [Mod::StacksOnStacks, Mod::Empty, Mod::Empty],
        ),
        Armour::new(
            ArmourName::Cloak,
            [Mod::TimeDilation, Mod::PowerfulAttraction, Mod::Reaper],
        ),
    ],
    stats_priority: [
        Stat::Class(70),
        Stat::Melee(200),
        Stat::Super(200),
        Stat::Grenade(200),
        Stat::Health(200),
        Stat::Weapons(200),
    ],
};
