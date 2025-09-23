use std::{collections::HashMap, fmt::Write, ops::Deref};

use bungie_api::{
    DestinyInventoryItemDefinition, DestinyPlugSetDefinition, types::destiny::DestinyItemType,
};
use google_sheets_api::types::sheet::{CellData, RowData};
use serde::{Deserialize, Serialize};
use serenity::all::{AutocompleteChoice, CreateEmbed, CreateEmbedAuthor, CreateEmbedFooter};

use super::{Affinity, Frame, Tier};

// const IDEAL_SHOTGUN_COLUMN: IdealWeaponColumns = IdealWeaponColumns {
//     column_1: Column1::BarrelShroud,
//     column_2: Column2::TacticalMag,
// };
// const IDEAL_SNIPER_COLUMN: IdealWeaponColumns = IdealWeaponColumns {
//     column_1: Column1::FlutedBarrel,
//     column_2: Column2::TacticalMag,
// };
// const IDEAL_FUSION_COLUMN: IdealWeaponColumns = IdealWeaponColumns {
//     column_1: Column1::FlutedBarrel,
//     column_2: Column2::AcceleratedCoils,
// };
// const IDEAL_BGL_COLUMN: IdealWeaponColumns = IdealWeaponColumns {
//     column_1: Column1::QuickLaunch,
//     column_2: Column2::SpikeGrenades,
// };
// const IDEAL_GLAIVE_COLUMN: IdealWeaponColumns = IdealWeaponColumns {
//     column_1: Column1::None,
//     column_2: Column2::None,
// };
// const IDEAL_TRACE_COLUMN: IdealWeaponColumns = IdealWeaponColumns {
//     column_1: Column1::Fluted Barrel,
//     column_2: Column2::Light Battery,
// };
// const IDEAL_ROCKET_SIDEARM_COLUMN: IdealWeaponColumns = IdealWeaponColumns {
//     column_1: Column1::VolatileLaunch,
//     column_2: Column2::HighExplosiveOrdnance,
// };
// const IDEAL_LMG_COLUMN: IdealWeaponColumns = IdealWeaponColumns {
//     column_1: Column1::FlutedBarrel,
//     column_2: Column2::ExtendedMag,
// };
// const IDEAL_HGL_COLUMN: IdealWeaponColumns = IdealWeaponColumns {
//     column_1: Column1::QuickLaunch,
//     column_2: Column2::SpikeGrenades,
// };
// const IDEAL_SWORD_COLUMN: IdealWeaponColumns = IdealWeaponColumns {
//     column_1: Column1::JaggedEdge,
//     column_2: Column2::SwordmastersGuard,
// };
// const IDEAL_ROCKET_COLUMN: IdealWeaponColumns = IdealWeaponColumns {
//     column_1: Column1::QuickLaunch,
//     column_2: Column2::ImpactCasing,
// };
// const IDEAL_LFR_COLUMN: IdealWeaponColumns = IdealWeaponColumns {
//     column_1: Column1::FlutedBarrel,
//     column_2: Column2::AcceleratedCoils,
// };

#[derive(Default)]
pub struct WeaponBuilder {
    pub name: String,
    pub archetype: String,
    pub affinity: String,
    pub frame: Option<String>,
    pub enhanceable: bool,
    pub shield: Option<u8>,
    pub reserves: Option<u16>,
    pub column_1: String,
    pub column_2: String,
    pub origin_trait: String,
    pub rank: u8,
    pub tier: Tier,
    pub notes: Option<String>,
}

impl WeaponBuilder {
    pub fn new(name: &str, archetype: impl Into<String>) -> Self {
        let name = match name {
            "Song of Ir Yut" => "Song of Ir Yût",
            "Fang of Ir Yut" => "Fang of Ir Yût",
            "Just In Case" => "Just in Case",
            "Braytech Osprey" => "BrayTech Osprey",
            "Braytech Werewolf" => "BrayTech Werewolf",
            "Arsenic Bite-4B" => "Arsenic Bite-4b",
            "Lunulata-4B" => "Lunulata-4b",
            "IKELOS_HC_V1.0.3" => "IKELOS_HC_v1.0.3",
            "IKELOS_SMG_V1.0.3" => "IKELOS_SMG_v1.0.3",
            "Elsie's Rifle" => "Elsie's Rifle",
            "Jararaca-3SR" => "Jararaca-3sr",
            "Redback-5SI" => "Redback-5si",
            "Judgement" => "Judgment",
            "Long Arm\nRotn version" => "Long Arm",
            name => name
                .trim()
                .trim_end_matches("\nBRAVE version")
                .trim_end_matches(" (BRAVE version)")
                .trim_end_matches("\nRotN version"),
        };

        WeaponBuilder {
            name: name.to_string(),
            archetype: archetype.into(),
            ..Default::default()
        }
    }

    pub fn affinity(mut self, affinity: impl Into<String>) -> Self {
        self.affinity = affinity.into();
        self
    }

    pub fn frame(mut self, frame: Option<impl Into<String>>) -> Self {
        self.frame = frame.map(|f| f.into());
        self
    }

    pub fn enhanceable(mut self, enhanceable: bool) -> Self {
        self.enhanceable = enhanceable;
        self
    }

    pub fn shield(mut self, shield: Option<u8>) -> Self {
        self.shield = shield;
        self
    }

    pub fn reserves(mut self, reserves: Option<u16>) -> Self {
        self.reserves = reserves;
        self
    }

    pub fn column_1(mut self, column_1: impl Into<String>) -> Self {
        self.column_1 = column_1.into();
        self
    }

    pub fn column_2(mut self, column_2: impl Into<String>) -> Self {
        self.column_2 = column_2.into();
        self
    }

    pub fn origin_trait(mut self, origin_trait: impl Into<String>) -> Self {
        self.origin_trait = origin_trait.into();
        self
    }

    pub fn rank(mut self, rank: u8) -> Self {
        self.rank = rank;
        self
    }

    pub fn tier(mut self, tier: impl Into<Tier>) -> Self {
        self.tier = tier.into();
        self
    }

    pub fn notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }

    pub fn from_row(name: &str, header: &RowData, row: RowData) -> Option<Self> {
        let mut data = header
            .values
            .iter()
            .zip(row.values)
            .map(|(h, r)| {
                (
                    h.formatted_value
                        .as_deref()
                        .unwrap_or_default()
                        .to_lowercase(),
                    r,
                )
            })
            .collect::<HashMap<String, CellData>>();

        let weapon_name = data.remove("name").unwrap().formatted_value.unwrap();

        if weapon_name == "Ideal" {
            return None;
        }

        let reserves = data
            .remove("reserves")
            .map(|r| r.formatted_value.unwrap())
            .filter(|s| !matches!(s.as_str(), "?" | "N/A" | "TBA"))
            .map(|s| {
                s.parse()
                    .unwrap_or_else(|_| panic!("Failed to parse: '{s}'"))
            });
        let shield = data
            .remove("shield")
            .map(|r| r.formatted_value.unwrap())
            .filter(|s| s != "?")
            .map(|s| s.parse().unwrap());
        let archetype = match name {
            "BGLs" | "HGLs" => String::from("Grenade Launcher"),
            "LMGs" => String::from("Machine Gun"),
            "LFRs" => String::from("Linear Fusion"),
            "HCs" => String::from("Hand Cannon"),
            "Other" => String::from("Other"),
            s => String::from(&s[..s.len() - 1]),
        };

        let mut weapon = Self::new(&weapon_name, archetype)
            .affinity(
                data.remove("affinity")
                    .or_else(|| data.remove("energy"))
                    .unwrap_or_else(|| panic!("affinity or energy should exist on data: {data:?}"))
                    .formatted_value
                    .unwrap_or_default(),
            )
            .frame(data.remove("frame").map(|f| f.formatted_value.unwrap()))
            .enhanceable(
                data.remove("enhance")
                    .or_else(|| data.remove("⬆\u{fe0f}"))
                    .unwrap_or_else(|| panic!("enhance should exist on data: {data:?}"))
                    .formatted_value
                    .unwrap()
                    == "Yes",
            )
            .shield(shield)
            .reserves(reserves)
            .column_1(data.remove("column 1").unwrap().formatted_value.unwrap())
            .column_2(data.remove("column 2").unwrap().formatted_value.unwrap())
            .origin_trait(
                data.remove("origin trait")
                    .unwrap()
                    .formatted_value
                    .unwrap(),
            )
            .rank(
                data.remove("rank")
                    .unwrap()
                    .formatted_value
                    .map(|s| s.parse().unwrap())
                    .unwrap_or_default(),
            )
            .tier(data.remove("tier").unwrap());

        if let Some(notes) = data.remove("notes").and_then(|d| d.formatted_value) {
            weapon = weapon.notes(notes);
        }

        Some(weapon)
    }

    pub fn build(self, item: &DestinyInventoryItemDefinition) -> Weapon {
        let icon = item
            .display_properties
            .icon
            .as_ref()
            .unwrap_or_else(|| panic!("No icon for: {}", self.name));

        Weapon {
            icon: icon.clone(),
            name: self.name,
            archetype: self.archetype,
            affinity: self.affinity.parse().unwrap(),
            frame: self.frame.map(|f| f.parse().unwrap()),
            enhanceable: self.enhanceable,
            reserves: self.reserves,
            column_1: self.column_1,
            column_2: self.column_2,
            origin_trait: self.origin_trait,
            rank: self.rank,
            tier: self.tier,
            notes: self.notes,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Weapon {
    pub icon: String,
    pub name: String,
    pub archetype: String,
    pub affinity: Affinity,
    pub frame: Option<Frame>,
    pub enhanceable: bool,
    pub reserves: Option<u16>,
    column_1: String,
    column_2: String,
    pub origin_trait: String,
    pub rank: u8,
    pub tier: Tier,
    pub notes: Option<String>,
}

impl Weapon {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn archetype(&self) -> &str {
        &self.archetype
    }

    pub fn perks(&self) -> Perks<'_> {
        let column_1 = self.column_1.split('\n').collect::<Vec<_>>();
        let column_2 = self.column_2.split('\n').collect::<Vec<_>>();

        Perks([column_1, column_2])
    }

    pub fn origin_trait(&self) -> &str {
        &self.origin_trait
    }

    pub fn as_api(
        &self,
        item_manifest: &HashMap<String, DestinyInventoryItemDefinition>,
        plug_manifest: &HashMap<String, DestinyPlugSetDefinition>,
    ) -> Vec<ApiWeapon> {
        let name = self.name().to_lowercase();

        let mut weapons = item_manifest
            .values()
            .filter(|item| {
                !matches!(
                    item.item_type,
                    DestinyItemType::None | DestinyItemType::Pattern | DestinyItemType::Dummy
                )
            })
            .filter(|item| {
                item.display_properties
                    .name
                    .to_lowercase()
                    .starts_with(&name)
            })
            .filter(|item| {
                item.inventory
                    .tier_type_name
                    .as_ref()
                    .is_some_and(|tier| tier.eq_ignore_ascii_case("Legendary"))
            })
            .filter_map(|item| item.sockets.as_ref().map(|sockets| (item, sockets)))
            .map(|(item, sockets)| {
                let plug_items = sockets.socket_entries.iter().map(|socket| {
                    match (
                        socket.randomized_plug_set_hash,
                        socket.reusable_plug_set_hash,
                        socket.reusable_plug_items.as_slice(),
                    ) {
                        (Some(hash), None, items) => {
                            let plug_set = plug_manifest.get(&hash.to_string()).unwrap();

                            plug_set
                                .reusable_plug_items
                                .iter()
                                .map(|item| item.plug_item_hash)
                                .chain(items.iter().map(|item| item.plug_item_hash))
                                .collect::<Vec<_>>()
                        }
                        (None, Some(hash), items) => {
                            let plug_set = plug_manifest.get(&hash.to_string()).unwrap();

                            plug_set
                                .reusable_plug_items
                                .iter()
                                .map(|item| item.plug_item_hash)
                                .chain(items.iter().map(|item| item.plug_item_hash))
                                .collect::<Vec<_>>()
                        }
                        (Some(random_hash), Some(reusable_hash), items) => {
                            let random_plug_set =
                                plug_manifest.get(&random_hash.to_string()).unwrap();
                            let reusable_hash =
                                plug_manifest.get(&reusable_hash.to_string()).unwrap();

                            random_plug_set
                                .reusable_plug_items
                                .iter()
                                .map(|item| item.plug_item_hash)
                                .chain(
                                    reusable_hash
                                        .reusable_plug_items
                                        .iter()
                                        .map(|item| item.plug_item_hash),
                                )
                                .chain(items.iter().map(|item| item.plug_item_hash))
                                .collect::<Vec<_>>()
                        }
                        (None, None, items) => items
                            .iter()
                            .map(|item| item.plug_item_hash)
                            .collect::<Vec<_>>(),
                    }
                });

                (item, plug_items)
            })
            .peekable();

        assert!(
            weapons.peek().is_some(),
            "At least 1 weapon should match: {name}"
        );

        let weapons = weapons
            .map(|(item, plug_items)| {
                let perks = plug_items
                    .map(|traits| {
                        let items = traits
                            .iter()
                            .map(|hash| hash.to_string())
                            .map(|hash| item_manifest.get(&hash).unwrap())
                            .filter(|item| !item.display_properties.name.is_empty())
                            .collect::<Vec<_>>();

                        items
                            .into_iter()
                            .filter(|perk_item| {
                                self.perks().0.iter().flatten().any(|perk| {
                                    perk_item.display_properties.name.eq_ignore_ascii_case(perk)
                                })
                            })
                            .map(|perk| perk.hash)
                            .collect::<Vec<_>>()
                    })
                    .filter(|perks| !perks.is_empty())
                    .collect::<Vec<_>>();

                (item.hash, perks)
            })
            .filter(|(_, perks)| !perks.is_empty())
            .map(|(hash, perks)| ApiWeapon {
                hash,
                perks: ApiPerks(perks),
            })
            .collect::<Vec<_>>();

        assert!(!weapons.is_empty(), "No weapon found for {name}");

        weapons
    }

    pub fn as_wishlist(
        &self,
        item_manifest: &HashMap<String, DestinyInventoryItemDefinition>,
        perk_manifest: &HashMap<String, DestinyPlugSetDefinition>,
    ) -> String {
        let weapons = self.as_api(item_manifest, perk_manifest);

        let mut s = format!("// {}\n//notes: tags:pve\n", self.name);

        let perks = weapons
            .into_iter()
            .map(|weapon| weapon.perks.as_wishlist(weapon.hash))
            .collect::<Vec<_>>()
            .join("\n");

        s.push_str(&perks);

        s
    }
}

impl<'a> From<&'a Weapon> for CreateEmbed<'a> {
    fn from(value: &'a Weapon) -> Self {
        let frame = value
            .frame
            .as_ref()
            .map(|f| format!("{f} "))
            .unwrap_or_default();

        let mut description = format!("Tier: {} (#{})", value.tier.tier(), value.rank);
        if let Some(reserves) = value.reserves {
            description.push_str(&format!("\nReserves: {reserves}"));
        }

        let mut embed = CreateEmbed::new()
            .author(CreateEmbedAuthor::new(format!(
                "{} {}{}",
                value.affinity,
                frame,
                value.archetype(),
            )))
            .title(value.name.to_string())
            .thumbnail(format!("https://www.bungie.net{}", value.icon))
            .footer(CreateEmbedFooter::new("From 'Destiny 2: Endgame Analysis'"))
            .colour(value.tier.colour)
            .description(description)
            .fields(
                value
                    .perks()
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (i + 1, p))
                    .map(|(i, p)| {
                        (
                            i,
                            p.iter()
                                .enumerate()
                                .map(|(i, line)| format!("{}. {}", i + 1, line))
                                .collect::<Vec<_>>(),
                        )
                    })
                    .map(|(i, p)| (format!("Perk {i}"), p.join("\n"), true)),
            )
            .field("Origin Trait", value.origin_trait(), false);

        if let Some(notes) = value.notes.as_deref() {
            let mut chars = notes.chars();
            let first_char = chars.next().unwrap();
            let notes = first_char.to_uppercase().to_string() + chars.as_str();
            embed = embed.field("Notes", notes, false);
        }

        embed
    }
}

impl From<Weapon> for AutocompleteChoice<'_> {
    fn from(value: Weapon) -> Self {
        AutocompleteChoice::new(value.name.clone(), value.name)
    }
}

pub struct Perks<'a>([Vec<&'a str>; 2]);

impl<'a> Deref for Perks<'a> {
    type Target = [Vec<&'a str>; 2];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct ApiWeapon {
    pub hash: u32,
    pub perks: ApiPerks,
}

#[derive(Debug, Clone)]
pub struct ApiPerks(Vec<Vec<u32>>);

impl ApiPerks {
    pub fn as_wishlist(&self, item_hash: u32) -> String {
        create_combinations_string(&self.0, item_hash)
    }
}

pub fn create_combinations_string(data: &[Vec<u32>], item_hash: u32) -> String {
    if data.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut current_combination = Vec::with_capacity(data.len());

    generate_combinations_iterative(data, 0, &mut current_combination, &mut result, item_hash);

    result
}

fn generate_combinations_iterative(
    data: &[Vec<u32>],
    depth: usize,
    current_combination: &mut Vec<u32>,
    output: &mut String,
    item_hash: u32,
) {
    if depth == data.len() {
        if !output.is_empty() {
            output.push('\n');
        }
        write!(output, "dimwishlist:item={}&perks=", item_hash).unwrap();
        for (i, &num) in current_combination.iter().enumerate() {
            if i > 0 {
                output.push(',');
            }
            write!(output, "{}", num).unwrap();
        }
        return;
    }

    for &num in &data[depth] {
        current_combination.push(num);
        generate_combinations_iterative(data, depth + 1, current_combination, output, item_hash);
        current_combination.pop();
    }
}
