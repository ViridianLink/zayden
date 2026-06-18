use super::super::titan::prismatic::{Abilities, Aspect, Grenade, Melee, Super};
use super::super::titan::{
    Arms,
    Chest,
    ClassAbility,
    Helmet,
    Jump,
    Legs,
    Mark,
    StoicismTrait,
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
    PrismaticFragment,
    Stat,
    TabletOfRuin,
    Weapon,
};

pub(crate) const PRISMATIC_TITAN: Loadout<'_> = Loadout {
    name: "Drengr's Lash",
    class: DestinyClass::Titan(Subclass::Prismatic {
        abilities: Abilities {
            super_: Super::Thundercrash,
            class: ClassAbility::RallyBarricade,
            jump: Jump::Catapult,
            melee: Melee::Thunderclap,
            grenade: Grenade::Pulse,
        },
        aspects: [
            Aspect::DrengrsLash([
                PrismaticFragment::Protection,
                PrismaticFragment::Courage,
                PrismaticFragment::Dominance,
            ]),
            Aspect::Knockout([PrismaticFragment::Justice, PrismaticFragment::Hope]),
        ],
    }),
    mode: Mode::PvE,
    tags: [None; 3],
    gear: Gear {
        weapons: [
            Some(Weapon::FestivalFlight([Perk::Slice, Perk::AttritionOrbs])),
            None,
            None,
        ],
        armour: Armour::Titan {
            helmet: Helmet::LuminopotentHelm([
                HelmetMod::Dynamo,
                HelmetMod::Dynamo,
                HelmetMod::PowerfulFriends,
            ]),
            arms: Arms::LuminopotentGauntlets([
                ArmsMod::Firepower,
                ArmsMod::StrandLoader,
                ArmsMod::BolsteringDetonation,
            ]),
            chest: Chest::LuminopotentPlate([
                ChestMod::ConcussiveDampener,
                ChestMod::Empty,
                ChestMod::Empty,
            ]),
            legs: Legs::LuminopotentGreaves([
                LegsMod::StrandScavenger,
                LegsMod::StacksOnStacks,
                LegsMod::Insulation,
            ]),
            mark: Mark::Stoicism([StoicismTrait::Abeyant, StoicismTrait::Horn], [
                ClassItemMod::PowerfulAttraction,
                ClassItemMod::ClassFont,
                ClassItemMod::TimeDilation,
            ]),
        },
        stats_priority: [
            Stat::Class(200),
            Stat::Super(200),
            Stat::Melee(200),
            Stat::Grenade(200),
            Stat::Weapons(200),
            Stat::Health(200),
        ],
    },
    artifact: Artifact::TabletOfRuin([
        Some(TabletOfRuin::Dielectric),
        Some(TabletOfRuin::VolatileMarksman),
        Some(TabletOfRuin::VileWeave),
        Some(TabletOfRuin::UnravelingOrbs),
        Some(TabletOfRuin::Flashover),
        Some(TabletOfRuin::DefibrillatingBlast),
        Some(TabletOfRuin::ToShreds),
    ]),
    details: Details::new("LlamaD2", "https://dim.gg/fhifcqy/Prismatic")
        .video("https://youtu.be/_XrSVh-ZIGg"),
};
