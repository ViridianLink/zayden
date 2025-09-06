use super::weapons::{IKELOS_SG_V103, NAVIGATOR};
use super::{
    Abilities, Armour, ArmourName, ArtifactPerk, Aspect, ClassAbility, DestinyClass, Details,
    Fragment, Gear, Grenade, Jump, Loadout, Melee, Mod, Mode, Stat, Subclass, SubclassType, Super,
    Tag,
};

pub const BOSS_PRISMATIC_HUNTER: Loadout = Loadout::new(
    "Grapple Melee",
    DestinyClass::Hunter,
    Mode::PvE,
    SUBCLASS,
    GEAR,
    Details::new("LlamaD2", "https://dim.gg/epy5f2q/Prismatic")
        .video("https://youtu.be/syoTkuT-s3w"),
)
.tags([Some(Tag::BossDamage), None, None])
.artifact([
    Some(ArtifactPerk::TightlyWoven),
    Some(ArtifactPerk::Shieldcrush),
    None,
    None,
    None,
    None,
    None,
]);

const SUBCLASS: Subclass = Subclass {
    subclass: SubclassType::Prismatic,
    abilities: ABILITIES,
    aspects: [Aspect::StylishExecutioner, Aspect::WintersShroud],
    fragments: [
        Some(Fragment::FacetOfProtection),
        Some(Fragment::FacetOfPurpose),
        Some(Fragment::FacetOfDawn),
        Some(Fragment::FacetOfCourage),
        None,
    ],
};

const ABILITIES: Abilities = Abilities {
    super_: Super::GoldenGunMarksman,
    class: ClassAbility::GamblersDodge,
    jump: Jump::Triple,
    melee: Melee::CombinationBlow,
    grenade: Grenade::Grapple,
};

const GEAR: Gear = Gear {
    weapons: [Some(NAVIGATOR), Some(IKELOS_SG_V103), None],
    armour: [
        Armour::new(
            ArmourName::CollectivePsycheCasque,
            [Mod::AshesToAssets, Mod::StrandSiphon, Mod::Empty],
        ),
        Armour::new(
            ArmourName::CollectivePsycheSleeves,
            [Mod::GrenadeFont, Mod::GrenadeFont, Mod::HeavyHanded],
        ),
        Armour::new(ArmourName::CollectivePsycheCuirass, [Mod::Empty; 3]),
        Armour::new(
            ArmourName::CollectivePsycheStrides,
            [Mod::StrandScavenger, Mod::StacksOnStacks, Mod::Empty],
        ),
        Armour::new(
            ArmourName::Relativism(("Inmost Light", "Verity")),
            [
                Mod::SpecialFinisher,
                Mod::PowerfulAttraction,
                Mod::TimeDilation,
            ],
        ),
    ],
    stats_priority: [
        Stat::Class(70),
        Stat::Grenade(200),
        Stat::Melee(200),
        Stat::Super(200),
        Stat::Health(200),
        Stat::Weapons(200),
    ],
};
