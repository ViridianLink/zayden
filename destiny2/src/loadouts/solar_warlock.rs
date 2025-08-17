use super::weapons::{MINT_RETROGRADE, SUNSHOT};
use super::{
    Abilities, Armour, ArmourName, ArtifactPerk, Aspect, ClassAbility, DestinyClass, Details,
    Fragment, Gear, Grenade, Jump, Loadout, Melee, Mod, Mode, Stat, Subclass, SubclassType, Super,
};

pub const SOLAR_WARLOCK: Loadout = Loadout {
    name: "Starfire Protocol",
    class: DestinyClass::Warlock,
    mode: Mode::PvE,
    tags: [None; 3],
    subclass: SUBCLASS,
    gear: GEAR,
    artifact: [
        Some(ArtifactPerk::FeverAndChill),
        Some(ArtifactPerk::RefreshThreads),
        Some(ArtifactPerk::CauterizedDarkness),
        Some(ArtifactPerk::RadiantShrapnel),
        Some(ArtifactPerk::Shieldcrush),
        None,
        None,
    ],
    details: Details::new("LlamaD2", "https://dim.gg/aelfwzq/Solar")
        .video("https://www.youtube.com/watch?v=AkDl3T_iIuc"),
};

const SUBCLASS: Subclass = Subclass {
    subclass: SubclassType::Solar,
    abilities: ABILITIES,
    aspects: [Aspect::TouchOfFlame, Aspect::Hellion],
    fragments: [
        Some(Fragment::EmberOfMercy),
        Some(Fragment::EmberOfEmpyrean),
        Some(Fragment::EmberOfTorches),
        Some(Fragment::EmberOfSearing),
        None,
    ],
};

const ABILITIES: Abilities = Abilities {
    super_: Super::SongOfFlame,
    class: ClassAbility::PhoenixDive,
    jump: Jump::BurstGlide,
    melee: Melee::IncineratorSnap,
    grenade: Grenade::Fusion,
};

const GEAR: Gear = Gear {
    weapons: [Some(MINT_RETROGRADE), Some(SUNSHOT), None],
    armour: [
        Armour::new(
            ArmourName::CollectivePsycheCover,
            [Mod::HarmonicSiphon, Mod::SuperFont, Mod::AshesToAssets],
        ),
        Armour::new(
            ArmourName::CollectivePsycheGloves,
            [Mod::GrenadeFont, Mod::GrenadeFont, Mod::Firepower],
        ),
        Armour::new(ArmourName::StarfireProtocol, [Mod::Empty; 3]),
        Armour::new(
            ArmourName::CollectivePsycheBoots,
            [Mod::StacksOnStacks, Mod::Innervation, Mod::StrandScavenger],
        ),
        Armour::new(
            ArmourName::CollectivePsycheBond,
            [Mod::TimeDilation, Mod::Reaper, Mod::PowerfulAttraction],
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
