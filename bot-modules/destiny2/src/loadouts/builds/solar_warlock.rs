use super::super::warlock::solar::{Abilities, Aspect, Melee, Super};
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
    LegsMod,
    Loadout,
    Mode,
    Perk,
    QueensfoilCenser,
    SolarFragment,
    SolarGrenade,
    Stat,
    Weapon,
};

pub const SOLAR_WARLOCK: Loadout<'_> = Loadout {
    name: "Boots of the Assembler",
    class: DestinyClass::Warlock(Subclass::Solar {
        abilities: Abilities {
            super_: Super::WellOfRadiance,
            class: ClassAbility::HealingRift,
            jump: Jump::BurstJump,
            melee: Melee::IncineratorSnap,
            grenade: SolarGrenade::Healing,
        },
        aspects: [
            Aspect::Hellion([SolarFragment::Singeing, SolarFragment::Benevolence]),
            Aspect::IcarusDash([
                SolarFragment::Ashes,
                SolarFragment::Torches,
                SolarFragment::Searing,
            ]),
        ],
    }),
    mode: Mode::PvE,
    tags: [None; 3],
    gear: Gear {
        weapons: [
            Some(Weapon::MintRetrograde([Perk::BeaconRounds, Perk::BaitAndSwitch])),
            Some(Weapon::ChoirOfOne(Perk::Onslaught)),
            None,
        ],
        armour: Armour::Warlock {
            helmet: Hood::Any([
                HelmetMod::SpecialAmmoFinder,
                HelmetMod::SpecialAmmoScout,
                HelmetMod::VoidSiphon,
            ]),
            gloves: Gloves::Any([
                ArmsMod::MeleeFont,
                ArmsMod::VoidLoader,
                ArmsMod::FocusingStrike,
            ]),
            robes: Robes::Any([
                ChestMod::ConcussiveDampener,
                ChestMod::Empty,
                ChestMod::Empty,
            ]),
            boots: Boots::BootsOfTheAssembler([
                LegsMod::StacksOnStacks,
                LegsMod::VoidScavenger,
                LegsMod::Empty,
            ]),
            bond: Bond::Any([
                ClassItemMod::ClassFont,
                ClassItemMod::TimeDilation,
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
    artifact: Artifact::QueensfoilCenser([
        Some(QueensfoilCenser::RaysOfPrecision),
        Some(QueensfoilCenser::RevitalizingBlast),
        None,
        None,
        None,
        None,
        None,
    ]),
    details: Details::new("LlamaD2", "https://dim.gg/la56xhi/Raid")
        .video("https://youtu.be/Gt2pLQvbZUA?t=944"),
};
