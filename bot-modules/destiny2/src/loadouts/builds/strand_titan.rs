use super::weapons::MONTE_CARLO;
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

pub(super) const STRAND_TITAN: Loadout<'_> = Loadout {
    name: "Flechette Storm",
    class: DestinyClass::Titan,
    mode: Mode::PvE,
    tags: [Some(Tag::AbilityFocused), None, None],
    subclass: SUBCLASS,
    gear: GEAR,
    artifact: Artifact::Unknown([
        Some(ArtifactPerk::TightlyWoven),
        Some(ArtifactPerk::RefreshThreads),
        Some(ArtifactPerk::ThreadedBlast),
        Some(ArtifactPerk::Shieldcrush),
        Some(ArtifactPerk::TangledWeb),
        None,
        None,
        None,
    ]),
    details: Details::new("LlamaD2", "https://dim.gg/g37nsna/Strand")
        .video("https://youtu.be/T7KhZa1sBuA"),
};

const SUBCLASS: Subclass = Subclass {
    kind: Subclass::Strand,
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
    jump: Jump::Catapult,
    melee: Melee::FrenziedBlade,
    grenade: Grenade::Grapple,
};

const GEAR: Gear<'_> = Gear {
    weapons: [Some(MONTE_CARLO), None, None],
    armour: [
        Armour::new(ArmourName::CollectivePsycheHelm, [
            Mod::KineticSiphon,
            Mod::HandsOn,
            Mod::HandsOn,
        ]),
        Armour::new(ArmourName::WishfulIgnorance, [
            Mod::HeavyHanded,
            Mod::MeleeFont,
            Mod::MeleeFont,
        ]),
        Armour::new(ArmourName::CollectivePsychePlate, [Mod::Empty; 3]),
        Armour::new(ArmourName::CollectivePsycheGreaves, [
            Mod::Invigoration,
            Mod::Recuperation,
            Mod::StacksOnStacks,
        ]),
        Armour::new(ArmourName::CollectivePsycheMark, [
            Mod::TimeDilation,
            Mod::Outreach,
            Mod::Reaper,
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
