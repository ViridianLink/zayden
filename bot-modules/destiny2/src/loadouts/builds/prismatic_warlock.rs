use super::weapons::CHOIR_OF_ONE;
use super::{
    Abilities,
    Armour,
    ArmourName,
    Artifact,
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
};

pub(super) const PRISMATIC_WARLOCK: Loadout<'_> = Loadout {
    name: "Lightning Surge",
    class: DestinyClass::Warlock,
    mode: Mode::PvE,
    tags: [None; 3],
    subclass: SUBCLASS,
    gear: GEAR,
    artifact: Artifact::Unknown([None; 8]),
    details: Details::new("OscarSix", "https://dim.gg/vizvlti/Lightning-Surge"),
};

const SUBCLASS: Subclass = Subclass {
    kind: Subclass::Prismatic,
    abilities: ABILITIES,
    aspects: [Aspect::LightningSurge, Aspect::FeedTheVoid],
    fragments: [
        Some(Fragment::FacetOfProtection),
        Some(Fragment::FacetOfDawn),
        Some(Fragment::FacetOfPurpose),
        Some(Fragment::FacetOfCourage),
        Some(Fragment::FacetOfDominance),
    ],
};

const ABILITIES: Abilities = Abilities {
    super_: Super::NovaBombCataclysm,
    class: ClassAbility::PhoenixDive,
    jump: Jump::BurstGlide,
    melee: Melee::ArcaneNeedle,
    grenade: Grenade::Vortex,
};

const GEAR: Gear<'_> = Gear {
    weapons: [None, Some(CHOIR_OF_ONE), None],
    armour: [
        Armour::new(ArmourName::WarlockHood, [
            Mod::HandsOn,
            Mod::HandsOn,
            Mod::Empty,
        ]),
        Armour::new(ArmourName::WarlockGloves, [
            Mod::MeleeFont,
            Mod::MeleeFont,
            Mod::HeavyHanded,
        ]),
        Armour::new(ArmourName::WarlockRobes, [Mod::Empty, Mod::Empty, Mod::Empty]),
        Armour::new(ArmourName::WarlockBoots, [
            Mod::Recuperation,
            Mod::Invigoration,
            Mod::Absolution,
        ]),
        Armour::new(ArmourName::Solipsism(("Inmost Light", "Synthoceps")), [
            Mod::TimeDilation,
            Mod::PowerfulAttraction,
            Mod::Outreach,
        ]),
    ],
    stats_priority: [
        Stat::Melee(200),
        Stat::Grenade(200),
        Stat::Super(200),
        Stat::Class(200),
        Stat::Weapons(200),
        Stat::Health(200),
    ],
};
