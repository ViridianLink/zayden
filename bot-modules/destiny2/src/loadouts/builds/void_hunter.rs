use super::super::hunter::void::{Abilities, Aspect, Melee, Super};
use super::super::hunter::{
    ClassAbility,
    Cloak,
    Gauntlets,
    Greaves,
    Helmet,
    Jump,
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
    LegsMod,
    Loadout,
    Mode,
    NpaRepulsionRegulator,
    Perk,
    Stat,
    VoidFragment,
    VoidGrenade,
    Weapon,
};

pub const VOID_HUNTER: Loadout<'_> = Loadout {
    name: "Soul Siphon",
    class: DestinyClass::Hunter(Subclass::Void {
        abilities: Abilities {
            super_: Super::ShadowshotMoebiusQuiver,
            class: ClassAbility::GamblersDodge,
            jump: Jump::TripleJump,
            melee: Melee::PhantomSurge,
            grenade: VoidGrenade::VortexGrenade,
        },
        aspects: [
            Aspect::StylishExecutioner([
                VoidFragment::Undermining,
                VoidFragment::Starvation,
            ]),
            Aspect::TrappersAmbush([
                VoidFragment::Dilation,
                VoidFragment::Persistence,
            ]),
        ],
    }),
    mode: Mode::PvE,
    tags: [None; 3],
    gear: Gear {
        weapons: [
            Some(Weapon::MintRetrograde([Perk::BeaconRounds, Perk::MasterOfArms])),
            Some(Weapon::ChoirOfOne(Perk::DestabilizingRounds)),
            None,
        ],
        armour: Armour::Hunter {
            helmet: Helmet::GravitonForfeit([
                HelmetMod::SpecialAmmoFinder,
                HelmetMod::SpecialAmmoFinder,
                HelmetMod::HarmonicSiphon,
            ]),
            gauntlets: Gauntlets::Any([
                ArmsMod::MeleeFont,
                ArmsMod::HarmonicLoader,
                ArmsMod::HeavyHanded,
            ]),
            vest: Vest::Any([
                ChestMod::ConcussiveDampener,
                ChestMod::HarmonicAmmoGeneration,
                ChestMod::Empty,
            ]),
            greaves: Greaves::Any([
                LegsMod::StacksOnStacks,
                LegsMod::StrandScavenger,
                LegsMod::HarmonicScavenger,
            ]),
            cloak: Cloak::Any([
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
    artifact: Artifact::NpaRepulsionRegulator([
        Some(NpaRepulsionRegulator::UntoTheBreach),
        Some(NpaRepulsionRegulator::ProtectiveBreach),
        Some(NpaRepulsionRegulator::Supernova),
        None,
        None,
        None,
        None,
    ]),
    details: Details::new("LlamaD2", "https://dim.gg/ilfadba/Void")
        .video("https://youtu.be/pKWrXJg1ees"),
};
