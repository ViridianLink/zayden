use super::super::titan::solar::{Abilities, Aspect, Melee, Super};
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
    LegsMod,
    Loadout,
    Mode,
    SolarFragment,
    SolarGrenade,
    Stat,
    TabletOfRuin,
};

pub(crate) const SOLAR_TITAN: Loadout<'_> = Loadout {
    name: "Hallowfire Heart",
    class: DestinyClass::Titan(Subclass::Solar {
        abilities: Abilities {
            super_: Super::HammerOfSol,
            class: ClassAbility::RallyBarricade,
            jump: Jump::Catapult,
            melee: Melee::ThrowingHammer,
            grenade: SolarGrenade::Thermite,
        },
        aspects: [
            Aspect::RoaringFlames([SolarFragment::Torches, SolarFragment::Char]),
            Aspect::SolInvictus([SolarFragment::Solace, SolarFragment::Ashes]),
        ],
    }),
    mode: Mode::PvE,
    tags: [None; 3],
    gear: Gear {
        weapons: [None; 3],
        armour: Armour::Titan {
            helmet: Helmet::WillbreakersWatch([
                HelmetMod::HandsOn,
                HelmetMod::HandsOn,
                HelmetMod::HarmonicSiphon,
            ]),
            arms: Arms::WillbreakersFists([
                ArmsMod::HeavyHanded,
                ArmsMod::MeleeFont,
                ArmsMod::MeleeFont,
            ]),
            chest: Chest::HallowfireHeart([
                ChestMod::ConcussiveDampener,
                ChestMod::Empty,
                ChestMod::Empty,
            ]),
            legs: Legs::SmokeJumperBoots([
                LegsMod::StacksOnStacks,
                LegsMod::Absolution,
                LegsMod::Innervation,
            ]),
            mark: Mark::SmokeJumperMark([
                ClassItemMod::PowerfulAttraction,
                ClassItemMod::TimeDilation,
                ClassItemMod::ClassFont,
            ]),
        },
        stats_priority: [
            Stat::Super(200),
            Stat::Melee(200),
            Stat::Grenade(200),
            Stat::Weapons(200),
            Stat::Class(200),
            Stat::Health(200),
        ],
    },
    artifact: Artifact::TabletOfRuin([
        Some(TabletOfRuin::Dielectric),
        Some(TabletOfRuin::VolatileMarksman),
        Some(TabletOfRuin::Flashover),
        Some(TabletOfRuin::ElementalSiphon),
        Some(TabletOfRuin::UnravelingOrbs),
        Some(TabletOfRuin::DefibrillatingBlast),
        Some(TabletOfRuin::LimitBreak),
    ]),
    details: Details::new("LlamaD2", "https://dim.gg/kuzm76q/Solar")
        .video("https://youtu.be/-m-BsUCm7Z4"),
};
