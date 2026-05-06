use super::weapons::NEW_MALPAIS;
use super::{
    Abilities, Armour, ArmourName, ArtifactPerk, Aspect, ClassAbility, DestinyClass, Details,
    Fragment, Gear, Grenade, Jump, Loadout, Melee, Mod, Mode, Stat, Subclass, SubclassType, Super,
};

pub const STRAND_WARLOCK: Loadout = Loadout::new(
    "Weavewalk",
    DestinyClass::Warlock,
    Mode::PvE,
    SUBCLASS,
    GEAR,
    Details::new("LlamaD2", "https://dim.gg/fiauzci/Void").video("https://youtu.be/TBbOiMWPIkE"),
)
.artifact([
    Some(ArtifactPerk::TightlyWoven),
    Some(ArtifactPerk::ThreadlingProliferation),
    Some(ArtifactPerk::ElementalBenevolence),
    Some(ArtifactPerk::RefreshThreads),
    Some(ArtifactPerk::PackTactics),
    Some(ArtifactPerk::ThreadedBlast),
    Some(ArtifactPerk::Shieldcrush),
    Some(ArtifactPerk::TangledWeb),
]);

const SUBCLASS: Subclass = Subclass {
    subclass: SubclassType::Strand,
    abilities: ABILITIES,
    aspects: [Aspect::Weavewalk, Aspect::WeaversCall],
    fragments: [
        Some(Fragment::ThreadOfMind),
        Some(Fragment::ThreadOfWarding),
        Some(Fragment::ThreadOfEvolution),
        Some(Fragment::ThreadOfGeneration),
        Some(Fragment::ThreadOfFury),
    ],
};

const ABILITIES: Abilities = Abilities {
    super_: Super::Needlestorm,
    class: ClassAbility::EmpoweringRift,
    jump: Jump::BurstGlide,
    melee: Melee::ArcaneNeedle,
    grenade: Grenade::Threadling,
};

const GEAR: Gear = Gear {
    weapons: [Some(NEW_MALPAIS), None, None],
    armour: [
        Armour::new(
            ArmourName::AionAdapterHood,
            [Mod::SuperFont, Mod::SpecialAmmoFinder, Mod::HarmonicSiphon],
        ),
        Armour::new(
            ArmourName::AionAdapterGloves,
            [Mod::MeleeFont, Mod::GrenadeFont, Mod::MomentumTransfer],
        ),
        Armour::new(
            ArmourName::AIONRenewalRobes,
            [Mod::StrandAmmoGeneration, Mod::Empty, Mod::Empty],
        ),
        Armour::new(
            ArmourName::Swarmers,
            [Mod::WeaponsFont, Mod::WeaponsFont, Mod::HarmonicScavenger],
        ),
        Armour::new(
            ArmourName::AIONRenewalBond,
            [Mod::TimeDilation, Mod::PowerfulAttraction, Mod::Reaper],
        ),
    ],
    stats_priority: [
        Stat::Weapons(200),
        Stat::Melee(100),
        Stat::Class(200),
        Stat::Grenade(200),
        Stat::Super(200),
        Stat::Health(200),
    ],
};
