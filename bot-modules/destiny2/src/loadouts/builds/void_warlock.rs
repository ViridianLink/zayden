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
    Perk,
    Stat,
    VoidFragment,
    VoidGrenade,
    Weapon,
};

pub(crate) const VOID_WARLOCK: Loadout<'_> = Loadout {
    name: "Soul Siphon",
    class: DestinyClass::Warlock(Subclass::Void {
        abilities: Abilities {
            super_: Super::Cataclysm,
            class: ClassAbility::HealingRift,
            jump: Jump::Burst,
            melee: Melee::PocketSingularity,
            grenade: VoidGrenade::VortexGrenade,
        },
        aspects: [
            Aspect::FeedTheVoid([
                VoidFragment::Persistence,
                VoidFragment::Vigilance,
            ]),
            Aspect::SoulSiphon([
                VoidFragment::Undermining,
                VoidFragment::Expulsion,
                VoidFragment::Provision,
            ]),
        ],
    }),
    mode: Mode::PvE,
    tags: [None; 3],
    gear: Gear {
        weapons: [
            Some(Weapon::CullsShadow(Perk::SoulfireZeal)),
            Some(Weapon::RecklessOracle([
                Perk::DestabilizingRounds,
                Perk::ChaosReshaped,
            ])),
            None,
        ],
        armour: Armour::Warlock {
            helmet: Hood::MaskOfDetestation([
                HelmetMod::HandsOn,
                HelmetMod::HandsOn,
                HelmetMod::HarmonicSiphon,
            ]),
            gloves: Gloves::WintersGuile([
                ArmsMod::HeavyHanded,
                ArmsMod::HeavyHanded,
                ArmsMod::MeleeFont,
            ]),
            robes: Robes::RobesOfDetestation([
                ChestMod::ConcussiveDampener,
                ChestMod::Empty,
                ChestMod::Empty,
            ]),
            boots: Boots::BootsOfDetestation([
                LegsMod::StacksOnStacks,
                LegsMod::Absolution,
                LegsMod::Invigoration,
            ]),
            bond: Bond::BondOfDetestation([
                ClassItemMod::TimeDilation,
                ClassItemMod::ClassFont,
                ClassItemMod::PowerfulAttraction,
            ]),
        },
        stats_priority: [
            Stat::Melee(200),
            Stat::Grenade(200),
            Stat::Super(200),
            Stat::Health(200),
            Stat::Weapons(200),
            Stat::Class(200),
        ],
    },
    artifact: Artifact::HuntersJournal([
        Some(HuntersJournal::EnergyDiffusionSubstrate),
        Some(HuntersJournal::SustainedFire),
        Some(HuntersJournal::TargetingAutoloader),
        Some(HuntersJournal::ElementalSiphon),
        Some(HuntersJournal::VoidHegemony),
        Some(HuntersJournal::ExpandingAbyss),
        Some(HuntersJournal::Shieldcrush),
    ]),
    details: Details::new("LlamaD2", "https://dim.gg/dchyh6y/Raid")
        .video("https://youtu.be/LyWoZXrUGTM"),
};
