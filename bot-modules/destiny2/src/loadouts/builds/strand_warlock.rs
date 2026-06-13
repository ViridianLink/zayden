use super::super::warlock::strand::{Abilities, Aspect, Melee, Super};
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
    ImplementOfCuriosity,
    LegsMod,
    Loadout,
    Mode,
    Stat,
    StrandFragment,
    StrandGrenade,
    Weapon,
};

pub const STRAND_WARLOCK: Loadout<'_> = Loadout {
    name: "Weavewalk",
    class: DestinyClass::Warlock(Subclass::Strand {
        abilities: Abilities {
            super_: Super::Needlestorm,
            class: ClassAbility::HealingRift,
            jump: Jump::BurstJump,
            melee: Melee::ArcaneNeedle,
            grenade: StrandGrenade::Grapple,
        },
        aspects: [
            Aspect::WeaversCall([StrandFragment::Fury, StrandFragment::Mind]),
            Aspect::Weavewalk([
                StrandFragment::Evolution,
                StrandFragment::Generation,
                StrandFragment::Propagation,
            ]),
        ],
    }),
    mode: Mode::PvE,
    tags: [None; 3],
    gear: Gear {
        weapons: [None, None, Some(Weapon::ServiceOfLuzaku)],
        armour: Armour::Warlock {
            helmet: Hood::Deimosuffusion([
                HelmetMod::Dynamo,
                HelmetMod::HarmonicSiphon,
                HelmetMod::HeavyAmmoFinder,
            ]),
            gloves: Gloves::Any([
                ArmsMod::HarmonicLoader,
                ArmsMod::MeleeFont,
                ArmsMod::Firepower,
            ]),
            robes: Robes::Any([
                ChestMod::ConcussiveDampener,
                ChestMod::HarmonicAmmoGeneration,
                ChestMod::Empty,
            ]),
            boots: Boots::Any([
                LegsMod::StacksOnStacks,
                LegsMod::HarmonicScavenger,
                LegsMod::Insulation,
            ]),
            bond: Bond::Any([
                ClassItemMod::PowerfulAttraction,
                ClassItemMod::ClassFont,
                ClassItemMod::TimeDilation,
            ]),
        },
        stats_priority: [
            Stat::Weapons(200),
            Stat::Super(200),
            Stat::Class(200),
            Stat::Grenade(200),
            Stat::Melee(200),
            Stat::Health(200),
        ],
    },
    artifact: Artifact::ImplementOfCuriosity([
        Some(ImplementOfCuriosity::PackTactics),
        None,
        None,
        None,
        None,
        None,
        None,
    ]),
    details: Details::new("LlamaD2", "https://dim.gg/fiauzci/Void")
        .video("https://youtu.be/TBbOiMWPIkE"),
};
