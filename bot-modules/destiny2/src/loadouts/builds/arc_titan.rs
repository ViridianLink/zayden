use super::super::titan::arc::{Abilities, Aspect, Melee, Super};
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
    name: "Crest of Alpha Lupi",
    class: DestinyClass::Titan(Subclass::Arc {
        abilities: Abilities {
            super_: Super::Thundercrash,
            class: ClassAbility::RallyBarricade,
            jump: Jump::Catapult,
            melee: Melee::Thunderclap,
            grenade: ArcGrenade::Pulse,
        },
        aspects: [
            Aspect::StormsKeep([ArcFragment::Shock, ArcFragment::Frequency]),
            Aspect::Knockout([ArcFragment::Resistance, ArcFragment::Ions]),
        ],
    }),
    mode: Mode::PvE,
    tags: [None; 3],
    gear: Gear {
        weapons: [
            Some(Weapon::MintRetrograde([Perk::FieldPrep, Perk::Slice])),
            Some(Weapon::ChoirOfOne(Perk::DestabilizingRounds)),
            None,
        ],
        armour: Armour::Titan {
            helmet: Helmet::WarNumensCrown([
                HelmetMod::SpecialAmmoFinder,
                HelmetMod::SpecialAmmoScout,
                HelmetMod::VoidSiphon,
            ]),
            arms: Arms::WarNumensFist([
                ArmsMod::HeavyHanded,
                ArmsMod::BolsteringDetonation,
                ArmsMod::VoidLoader,
            ]),
            chest: Chest::CrestOfAlphaLupi([
                ChestMod::ConcussiveDampener,
                ChestMod::Empty,
                ChestMod::Empty,
            ]),
            legs: Legs::PromisedReunionGreaves([
                LegsMod::StacksOnStacks,
                LegsMod::StrandScavenger,
                LegsMod::Invigoration,
            ]),
            mark: Mark::PromisedReunionMark([
                ClassItemMod::SpecialFinisher,
                ClassItemMod::PowerfulAttraction,
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
        Some(TabletOfRuin::Dielectric),
        Some(TabletOfRuin::VolatileMarksman),
        Some(TabletOfRuin::MalignedHarvest),
        Some(TabletOfRuin::GoldFromLead),
        Some(TabletOfRuin::Flashover),
        Some(TabletOfRuin::DefibrillatingBlast),
        Some(TabletOfRuin::ToShreds),
    ]),
    details: Details::new("LlamaD2", "https://dim.gg/go2ivrq/Raid")
        .video("https://youtu.be/PIS5RRBkn0E"),
};
