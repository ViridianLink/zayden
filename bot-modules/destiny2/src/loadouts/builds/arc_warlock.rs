use super::super::warlock::arc::{Abilities, Aspect, Melee, Super};
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
    Stat,
    TabletOfRuin,
    Weapon,
};

pub(crate) const ARC_WARLOCK: Loadout<'_> = Loadout {
    name: "Geomag Super Spam",
    class: DestinyClass::Warlock(Subclass::Arc {
        abilities: Abilities {
            super_: Super::ChaosReach,
            class: ClassAbility::HealingRift,
            jump: Jump::Burst,
            melee: Melee::ChainLightning,
            grenade: ArcGrenade::Pulse,
        },
        aspects: [
            Aspect::ElectrostaticMind([ArcFragment::Shock, ArcFragment::Resistance]),
            Aspect::IonicSentry([ArcFragment::Discharge, ArcFragment::Frequency]),
        ],
    }),
    mode: Mode::PvE,
    tags: [None; 3],
    gear: Gear {
        weapons: [None, Some(Weapon::DelicateTomb), None],
        armour: Armour::Warlock {
            helmet: Hood::LuminopotentCover([
                HelmetMod::SpecialAmmoFinder,
                HelmetMod::SuperFont,
                HelmetMod::HarmonicSiphon,
            ]),
            gloves: Gloves::LuminopotentGloves([
                ArmsMod::GrenadeFont,
                ArmsMod::BolsteringDetonation,
                ArmsMod::HarmonicLoader,
            ]),
            robes: Robes::LuminopotentRobes([
                ChestMod::ConcussiveDampener,
                ChestMod::Empty,
                ChestMod::Empty,
            ]),
            boots: Boots::GeomagStabilizers([
                LegsMod::ElementalCharge,
                LegsMod::WeaponsFont,
                LegsMod::HarmonicScavenger,
            ]),
            bond: Bond::LuminopotentBond([
                ClassItemMod::ClassFont,
                ClassItemMod::PowerfulAttraction,
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
    artifact: Artifact::TabletOfRuin([
        Some(TabletOfRuin::Dielectric),
        Some(TabletOfRuin::ElementalSiphon),
        Some(TabletOfRuin::GoldFromLead),
        Some(TabletOfRuin::PhotonicFlare),
        Some(TabletOfRuin::Flashover),
        Some(TabletOfRuin::DefibrillatingBlast),
        Some(TabletOfRuin::LimitBreak),
    ]),
    details: Details::new("LlamaD2", "https://dim.gg/i2kny6a/Arc")
        .video("https://youtu.be/sFzAdAl3ULw"),
};
