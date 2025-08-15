use super::weapons::{IKELOS_SG_V103, NAVIGATOR};
use super::{
    Abilities, Armour, ArtifactPerk, Aspect, ClassAbility, DestinyClass, Details, Fragment, Gear,
    Grenade, Jump, Loadout, Melee, Mod, Mode, Stat, Subclass, SubclassType, Super, Tag,
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
            "Collective Psyche Casque",
            [Mod::AshesToAssets, Mod::StrandSiphon, Mod::Empty],
        ),
        Armour::new(
            "Collective Psyche Sleeves",
            [Mod::GrenadeFont, Mod::GrenadeFont, Mod::HeavyHanded],
        ),
        Armour::new("Collective Psyche Cuirass", [Mod::Empty; 3]),
        Armour::new(
            "Collective Psyche Strides",
            [Mod::StrandScavenger, Mod::StacksOnStacks, Mod::Empty],
        ),
        Armour::new(
            "Relativism (Inmost Light + Verity)",
            [
                Mod::SpecialFinisher,
                Mod::PowerfulAttraction,
                Mod::TimeDilation,
            ],
        ),
    ],
    stats_priority: [
        Stat::Grenade,
        Stat::Melee,
        Stat::Class,
        Stat::Super,
        Stat::Health,
        Stat::Weapons,
    ],
};
