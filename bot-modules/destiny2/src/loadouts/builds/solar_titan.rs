use super::super::titan::solar::{Abilities, Aspect, Melee, Super};
use super::super::titan::{
    ClassAbility,
    Gauntlets,
    Greaves,
    Helmet,
    Jump,
    Mark,
    Plate,
    Subclass,
};
use super::super::{
    Armour,
    ArmsMod,
    Artifact,
    ChestMod,
    ClassItemMod,
    DestinyClass,
    Details,
    EncryptedDataDisk,
    Gear,
    HelmetMod,
    LegsMod,
    Loadout,
    Mode,
    Perk,
    SolarFragment,
    SolarGrenade,
    Stat,
    Weapon,
};

pub const SOLAR_TITAN: Loadout<'_> = Loadout {
    name: "Fusion Grenades",
    class: DestinyClass::Titan(Subclass::Solar {
        abilities: Abilities {
            super_: Super::HammerOfSol,
            class: ClassAbility::RallyBarricade,
            jump: Jump::CatapultLift,
            melee: Melee::HammerStrike,
            grenade: SolarGrenade::FusionGrenade,
        },
        aspects: [
            Aspect::RoaringFlames([
                SolarFragment::EmberOfChar,
                SolarFragment::EmberOfAshes,
            ]),
            Aspect::SolInvictus([
                SolarFragment::EmberOfResolve,
                SolarFragment::EmberOfSearing,
            ]),
        ],
    }),
    mode: Mode::PvE,
    tags: [None; 3],
    gear: Gear {
        weapons: [
            Some(Weapon::PraxicBlade([Perk::RangedWeapon, Perk::CormorantCombo])),
            Some(Weapon::YeartideApex([Perk::HealClip, Perk::ChaosReshaped])),
            None,
        ],
        armour: Armour::Titan {
            helmet: Helmet::Any([
                HelmetMod::AshesToAssets,
                HelmetMod::HandsOn,
                HelmetMod::KineticSiphon,
            ]),
            gauntlets: Gauntlets::AshenWake([
                ArmsMod::Firepower,
                ArmsMod::HeavyHanded,
                ArmsMod::GrenadeFont,
            ]),
            plate: Plate::Any([
                ChestMod::ConcussiveDampener,
                ChestMod::Empty,
                ChestMod::Empty,
            ]),
            greaves: Greaves::Any([
                LegsMod::StacksOnStacks,
                LegsMod::Innervation,
                LegsMod::Absolution,
            ]),
            mark: Mark::Any([
                ClassItemMod::TimeDilation,
                ClassItemMod::PowerfulAttraction,
                ClassItemMod::Reaper,
            ]),
        },
        stats_priority: [
            Stat::Grenade(200),
            Stat::Super(200),
            Stat::Melee(200),
            Stat::Weapons(200),
            Stat::Class(200),
            Stat::Health(200),
        ],
    },
    artifact: Artifact::EncryptedDataDisk([
        Some(EncryptedDataDisk::KineticSynthesis),
        Some(EncryptedDataDisk::PowerFromPain),
        Some(EncryptedDataDisk::Dielectric),
        Some(EncryptedDataDisk::CombinationArgentBlade),
        Some(EncryptedDataDisk::SingularityBlade),
        Some(EncryptedDataDisk::SnipersMeditation),
        None,
    ]),
    details: Details::new("LlamaD2", "https://dim.gg/iti7lfa/Solar")
        .video("https://youtu.be/IhBfmN00LEs?t=820"),
};
