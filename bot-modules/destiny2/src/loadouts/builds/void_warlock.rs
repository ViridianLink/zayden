use super::super::warlock::void::{Abilities, Aspect, Melee, Super};
use super::super::warlock::{
    Bond,
    Boots,
    ClassAbility,
    Gloves,
    Hood,
    Jump,
    Robes,
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
    Gear,
    HelmetMod,
    HuntersJournal,
    LegsMod,
    Loadout,
    Mode,
    Stat,
    VoidFragment,
    VoidGrenade,
    Weapon,
};

pub const VOID_WARLOCK: Loadout<'_> = Loadout {
    name: "Soul Siphon",
    class: DestinyClass::Warlock(Subclass::Void {
        abilities: Abilities {
            super_: Super::NovaBombCataclysm,
            class: ClassAbility::HealingRift,
            jump: Jump::BurstJump,
            melee: Melee::PocketSingularity,
            grenade: VoidGrenade::VortexGrenade,
        },
        aspects: [
            Aspect::FeedTheVoid([
                VoidFragment::Vigilance,
                VoidFragment::Undermining,
            ]),
            Aspect::SoulSiphon([
                VoidFragment::Persistence,
                VoidFragment::Provision,
                VoidFragment::Remnants,
            ]),
        ],
    }),
    mode: Mode::PvE,
    tags: [None; 3],
    gear: Gear {
        weapons: [Some(Weapon::BadJuju), None, None],
        armour: Armour::Warlock {
            helmet: Hood::SkullOfDireAhamkara([
                HelmetMod::HandsOn,
                HelmetMod::KineticSiphon,
                HelmetMod::RadiantLight,
            ]),
            gloves: Gloves::Any([
                ArmsMod::MeleeFont,
                ArmsMod::MeleeFont,
                ArmsMod::HeavyHanded,
            ]),
            robes: Robes::Any([
                ChestMod::ConcussiveDampener,
                ChestMod::Empty,
                ChestMod::Empty,
            ]),
            boots: Boots::Any([
                LegsMod::StacksOnStacks,
                LegsMod::Invigoration,
                LegsMod::Empty,
            ]),
            bond: Bond::Any([
                ClassItemMod::PowerfulAttraction,
                ClassItemMod::TimeDilation,
                ClassItemMod::Outreach,
            ]),
        },
        stats_priority: [
            Stat::Melee(200),
            Stat::Super(200),
            Stat::Grenade(200),
            Stat::Health(200),
            Stat::Class(200),
            Stat::Weapons(200),
        ],
    },
    artifact: Artifact::HuntersJournal([
        Some(HuntersJournal::Shieldcrush),
        Some(HuntersJournal::VoidHegemony),
        None,
        None,
        None,
        None,
        None,
    ]),
    details: Details::new("LlamaD2", "https://dim.gg/fiauzci/Void")
        .video("https://youtu.be/TBbOiMWPIkE"),
};
