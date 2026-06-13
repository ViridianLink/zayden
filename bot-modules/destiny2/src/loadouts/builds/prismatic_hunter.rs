use super::super::hunter::prismatic::{Abilities, Aspect, Grenade, Melee, Super};
use super::super::hunter::{
    ClassAbility,
    Cloak,
    Gauntlets,
    Helmet,
    Jump,
    Legs,
    RelativismTrait,
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
    Gear,
    HelmetMod,
    HuntersJournal,
    LegsMod,
    Loadout,
    Mode,
    PrismaticFragment,
    Stat,
    Weapon,
};

pub(crate) const PRISMATIC_HUNTER: Loadout<'_> = Loadout {
    name: "Ascension",
    class: DestinyClass::Hunter(Subclass::Prismatic {
        abilities: Abilities {
            super_: Super::GoldenGunMarksman,
            class: ClassAbility::GamblersDodge,
            jump: Jump::Triple,
            melee: Melee::ThreadedSpike,
            grenade: Grenade::Grapple,
        },
        aspects: [
            Aspect::Ascension([PrismaticFragment::Courage, PrismaticFragment::Hope]),
            Aspect::GunpowderGamble([
                PrismaticFragment::Blessing,
                PrismaticFragment::Purpose,
                PrismaticFragment::Dawn,
            ]),
        ],
    }),
    mode: Mode::PvE,
    tags: [None; 3],
    gear: Gear {
        weapons: [Some(Weapon::MonteCarlo), None, None],
        armour: Armour::Hunter {
            helmet: Helmet::Any([
                HelmetMod::HandsOn,
                HelmetMod::HandsOn,
                HelmetMod::KineticSiphon,
            ]),
            gauntlets: Gauntlets::Any([
                ArmsMod::MeleeFont,
                ArmsMod::FocusingStrike,
                ArmsMod::HeavyHanded,
            ]),
            vest: Vest::Any([
                ChestMod::ConcussiveDampener,
                ChestMod::Empty,
                ChestMod::Empty,
            ]),
            legs: Legs::Any([
                LegsMod::StacksOnStacks,
                LegsMod::ArcScavenger,
                LegsMod::Innervation,
            ]),
            cloak: Cloak::Relativism(
                [RelativismTrait::Caliban, RelativismTrait::Synthoceps],
                [
                    ClassItemMod::ClassFont,
                    ClassItemMod::TimeDilation,
                    ClassItemMod::PowerfulAttraction,
                ],
            ),
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
    artifact: Artifact::HuntersJournal([
        Some(HuntersJournal::SolarFulmination),
        Some(HuntersJournal::Shieldcrush),
        None,
        None,
        None,
        None,
        None,
    ]),
    details: Details::new("LlamaD2", "https://dim.gg/d2dwroy/Prismatic")
        .video("https://youtu.be/pKWrXJg1ees?t=1160"),
};
