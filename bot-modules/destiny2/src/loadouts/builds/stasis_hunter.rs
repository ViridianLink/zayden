use super::super::hunter::stasis::{Abilities, Aspect, Melee, Super};
use super::super::hunter::{
    ClassAbility,
    Cloak,
    Gauntlets,
    Greaves,
    Helmet,
    Jump,
    Subclass,
    Vest,
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
    StasisFragment,
    StasisGrenade,
    Stat,
    Weapon,
};

pub const STASIS_HUNTER: Loadout<'_> = Loadout {
    name: "",
    class: DestinyClass::Hunter(Subclass::Stasis {
        abilities: Abilities {
            super_: Super::SilenceAndSquall,
            class: ClassAbility::GamblersDodge,
            jump: Jump::TripleJump,
            melee: Melee::WitheringBlade,
            grenade: StasisGrenade::Shatter,
        },
        aspects: [
            Aspect::GrimHarvest([StasisFragment::Rending, StasisFragment::Shards]),
            Aspect::TouchOfWinter([
                StasisFragment::Torment,
                StasisFragment::Conduction,
            ]),
        ],
    }),
    mode: Mode::PvE,
    tags: [None; 3],
    gear: Gear {
        weapons: [Some(Weapon::Khvostov7G0X), None, None],
        armour: Armour::Hunter {
            helmet: Helmet::Any([
                HelmetMod::SpecialAmmoFinder,
                HelmetMod::AshesToAssets,
                HelmetMod::KineticSiphon,
            ]),
            gauntlets: Gauntlets::Any([
                ArmsMod::MeleeFont,
                ArmsMod::GrenadeFont,
                ArmsMod::ImpactInduction,
            ]),
            vest: Vest::Any([
                ChestMod::ConcussiveDampener,
                ChestMod::Empty,
                ChestMod::Empty,
            ]),
            greaves: Greaves::FortunesFavor([
                LegsMod::ElementalCharge,
                LegsMod::Absolution,
                LegsMod::Innervation,
            ]),
            cloak: Cloak::Any([
                ClassItemMod::PowerfulAttraction,
                ClassItemMod::ClassFont,
                ClassItemMod::TimeDilation,
            ]),
        },
        stats_priority: [
            Stat::Weapons(200),
            Stat::Super(200),
            Stat::Grenade(200),
            Stat::Class(200),
            Stat::Melee(200),
            Stat::Health(200),
        ],
    },
    artifact: Artifact::EncryptedDataDisk([
        Some(EncryptedDataDisk::PowerFromPain),
        Some(EncryptedDataDisk::SingularityBlade),
        Some(EncryptedDataDisk::VoidInfestation),
        None,
        None,
        None,
        None,
    ]),
    details: Details::new("LlamaD2", "https://dim.gg/6bl3oay/Stasis")
        .video("https://youtu.be/pKWrXJg1ees?t=700"),
};
