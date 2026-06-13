use super::super::titan::void::{Abilities, Aspect, Melee, Super};
use super::super::titan::{
    Arms,
    Chest,
    ClassAbility,
    Helmet,
    Jump,
    Legs,
    Mark,
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

pub(crate) const VOID_TITAN: Loadout<'_> = Loadout {
    name: "Shield Bash",
    class: DestinyClass::Titan(Subclass::Void {
        abilities: Abilities {
            super_: Super::TwilightArsenal,
            class: ClassAbility::RallyBarricade,
            jump: Jump::Catapult,
            melee: Melee::ShieldBash,
            grenade: VoidGrenade::VortexGrenade,
        },
        aspects: [
            Aspect::Bastion([
                VoidFragment::Starvation,
                VoidFragment::Undermining,
                VoidFragment::Persistence,
            ]),
            Aspect::OffensiveBulwark([
                VoidFragment::Provision,
                VoidFragment::Leeching,
            ]),
        ],
    }),
    mode: Mode::PvE,
    tags: [None; 3],
    gear: Gear {
        weapons: [
            None,
            Some(Weapon::LotusEater([
                Perk::RepulsorBrace,
                Perk::DestabilizingRounds,
            ])),
            None,
        ],
        armour: Armour::Titan {
            helmet: Helmet::Any([
                HelmetMod::HandsOn,
                HelmetMod::HandsOn,
                HelmetMod::HarmonicSiphon,
            ]),
            arms: Arms::Any([
                ArmsMod::HeavyHanded,
                ArmsMod::HeavyHanded,
                ArmsMod::HeavyHanded,
            ]),
            chest: Chest::Any([
                ChestMod::ConcussiveDampener,
                ChestMod::Empty,
                ChestMod::Empty,
            ]),
            legs: Legs::PeregrineGreaves([
                LegsMod::StacksOnStacks,
                LegsMod::Absolution,
                LegsMod::Invigoration,
            ]),
            mark: Mark::Any([
                ClassItemMod::PowerfulAttraction,
                ClassItemMod::ClassFont,
                ClassItemMod::ClassFont,
            ]),
        },
        stats_priority: [
            Stat::Melee(200),
            Stat::Super(200),
            Stat::Health(200),
            Stat::Grenade(200),
            Stat::Class(200),
            Stat::Weapons(200),
        ],
    },
    artifact: Artifact::HuntersJournal([
        Some(HuntersJournal::Shieldcrush),
        Some(HuntersJournal::ExpandingAbyss),
        Some(HuntersJournal::VoidHegemony),
        None,
        None,
        None,
        None,
    ]),
    details: Details::new("LlamaD2", "https://dim.gg/3amqc4y/Void")
        .video("https://youtu.be/IhBfmN00LEs"),
};
