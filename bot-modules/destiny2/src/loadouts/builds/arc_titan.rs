use super::super::titan::arc::{Abilities, Aspect, Melee, Super};
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
    ArcFragment,
    ArcGrenade,
    Armour,
    ArmsMod,
    Artifact,
    ChestMod,
    ClassItemMod,
    DestinyClass,
    Details,
    Gear,
    HelmetMod,
    LegsMod,
    Loadout,
    Mode,
    Perk,
    Stat,
    TabletOfRuin,
    Weapon,
};

pub(crate) const ARC_TITAN: Loadout<'_> = Loadout {
    name: "Heart of Innmost Light",
    class: DestinyClass::Titan(Subclass::Arc {
        abilities: Abilities {
            super_: Super::Thundercrash,
            class: ClassAbility::RallyBarricade,
            jump: Jump::Catapult,
            melee: Melee::Thunderclap,
            grenade: ArcGrenade::Pulse,
        },
        aspects: [
            Aspect::StormsKeep([ArcFragment::Magnitude, ArcFragment::Frequency]),
            Aspect::Knockout([ArcFragment::Resistance, ArcFragment::Shock]),
        ],
    }),
    mode: Mode::PvE,
    tags: [None; 3],
    gear: Gear {
        weapons: [
            None,
            Some(Weapon::RecklessOracle([
                Perk::DestabilizingRounds,
                Perk::OneForAll,
            ])),
            None,
        ],
        armour: Armour::Titan {
            helmet: Helmet::Any([
                HelmetMod::SpecialAmmoFinder,
                HelmetMod::SpecialAmmoScout,
                HelmetMod::VoidSiphon,
            ]),
            gauntlets: Gauntlets::Any([
                ArmsMod::VoidLoader,
                ArmsMod::BolsteringDetonation,
                ArmsMod::Firepower,
            ]),
            plate: Plate::HeartOfInmostLight([
                ChestMod::ConcussiveDampener,
                ChestMod::Empty,
                ChestMod::Empty,
            ]),
            greaves: Greaves::Any([
                LegsMod::StacksOnStacks,
                LegsMod::Recuperation,
                LegsMod::Insulation,
            ]),
            mark: Mark::Any([
                ClassItemMod::SpecialFinisher,
                ClassItemMod::Reaper,
                ClassItemMod::PowerfulAttraction,
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
    artifact: Artifact::TabletOfRuin([
        Some(TabletOfRuin::VolatileMarksman),
        Some(TabletOfRuin::MalignedHarvest),
        Some(TabletOfRuin::Dielectric),
        Some(TabletOfRuin::Flashover),
        Some(TabletOfRuin::DefibrillatingBlast),
        None,
        None,
    ]),
    details: Details::new("LlamaD2", "https://dim.gg/bht5ddi/Arc")
        .video("https://youtu.be/IhBfmN00LEs?t=483"),
};
