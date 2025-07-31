use super::weapons::THIRD_ITERATION;
use super::{
    Abilities, Armour, ArtifactPerk, Aspect, ClassAbility, DestinyClass, Details, Fragment, Gear,
    Grenade, Jump, Loadout, Melee, Mod, Mode, Stat, Subclass, SubclassType, Super, Tag,
};

pub const PRISMATIC_HUNTER: Loadout = Loadout::new(
    "Ascension",
    DestinyClass::Hunter,
    Mode::PvE,
    SUBCLASS,
    GEAR,
    Details::new("LlamaD2", "https://dim.gg/x6wkejy/Prismatic")
        .video("https://youtu.be/Cqe3VZew2Vc"),
)
.tags([Some(Tag::AbilityFocused), None, None])
.artifact([
    Some(ArtifactPerk::AntiBarrierScoutAndPulse),
    Some(ArtifactPerk::TightlyWoven),
    Some(ArtifactPerk::RapidPrecisionRifling),
    Some(ArtifactPerk::ElementalBenevolence),
    Some(ArtifactPerk::ElementalCoalescence),
    Some(ArtifactPerk::Shieldcrush),
    Some(ArtifactPerk::TangledWeb),
]);

const SUBCLASS: Subclass = Subclass {
    subclass: SubclassType::Prismatic,
    abilities: ABILITIES,
    aspects: [Aspect::Ascension, Aspect::GunpowderGamble],
    fragments: [
        Some(Fragment::FacetOfHope),
        Some(Fragment::FacetOfProtection),
        Some(Fragment::FacetOfPurpose),
        Some(Fragment::FacetOfDawn),
        Some(Fragment::FacetOfBlessing),
    ],
};

const ABILITIES: Abilities = Abilities {
    super_: Super::GoldenGunMarksman,
    class: ClassAbility::MarksmansDodge,
    jump: Jump::Triple,
    melee: Melee::ThreadedSpike,
    grenade: Grenade::Grapple,
};

const GEAR: Gear = Gear {
    weapons: [None, Some(THIRD_ITERATION), None],
    armour: [
        Armour::new(
            "Bushido Cowl",
            [Mod::AshesToAssets, Mod::SuperFont, Mod::VoidSiphon],
        ),
        Armour::new(
            "Bushido Grips",
            [Mod::Firepower, Mod::GrenadeFont, Mod::FocusingStrike],
        ),
        Armour::new("Last Discipline Vest", [Mod::Empty, Mod::Empty, Mod::Empty]),
        Armour::new(
            "Last Discipline Strides",
            [Mod::Recuperation, Mod::StacksOnStacks, Mod::Invigoration],
        ),
        Armour::new(
            "Relativism (Inmost Light + Cyrtarachne)",
            [Mod::TimeDilation, Mod::ClassFont, Mod::PowerfulAttraction],
        ),
    ],
    stats_priority: [
        Stat::Grenade,
        Stat::Super,
        Stat::Melee,
        Stat::Class,
        Stat::Health,
        Stat::Weapons,
    ],
};
