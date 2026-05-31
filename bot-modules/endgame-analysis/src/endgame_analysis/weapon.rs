use std::collections::HashMap;
use std::fmt::Write;
use std::ops::Deref;

use bungie_api::types::destiny::DestinyItemType;
use bungie_api::{DestinyInventoryItemDefinition, DestinyPlugSetDefinition};
use google_sheets_api::types::sheet::{CellData, RowData};
use serde::{Deserialize, Serialize};
use serenity::all::{
    AutocompleteChoice,
    CreateEmbed,
    CreateEmbedAuthor,
    CreateEmbedFooter,
};
use tracing::error;

use super::{Affinity, Frame, Tier};

#[derive(Default)]
pub struct WeaponBuilder {
    pub name: String,
    pub archetype: String,
    pub affinity: String,
    pub frame: Option<String>,
    pub enhanceable: bool,
    pub shield: Option<u8>,
    pub reserves: Option<u16>,
    pub barrel: String,
    pub magazine: String,
    pub perk_1: String,
    pub perk_2: String,
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

        Self {
            name: name.to_string(),
            archetype: archetype.into(),
            ..Default::default()
        }
    }

    #[must_use]
    pub fn affinity(mut self, affinity: String) -> Self {
        self.affinity = affinity;
        self
    }

    #[must_use]
    pub fn frame(mut self, frame: Option<String>) -> Self {
        self.frame = frame;
        self
    }

    #[must_use]
    pub const fn enhanceable(mut self, enhanceable: bool) -> Self {
        self.enhanceable = enhanceable;
        self
    }

    #[must_use]
    pub const fn shield(mut self, shield: Option<u8>) -> Self {
        self.shield = shield;
        self
    }

    #[must_use]
    pub const fn reserves(mut self, reserves: Option<u16>) -> Self {
        self.reserves = reserves;
        self
    }

    #[must_use]
    pub fn barrel(mut self, barrel: String) -> Self {
        self.barrel = barrel;
        self
    }

    #[must_use]
    pub fn magazine(mut self, magazine: String) -> Self {
        self.magazine = magazine;
        self
    }

    #[must_use]
    pub fn perk_1(mut self, column_1: String) -> Self {
        self.perk_1 = column_1;
        self
    }

    #[must_use]
    pub fn perk_2(mut self, column_2: String) -> Self {
        self.perk_2 = column_2;
        self
    }

    #[must_use]
    pub fn origin_trait(mut self, origin_trait: String) -> Self {
        self.origin_trait = origin_trait;
        self
    }

    #[must_use]
    pub const fn rank(mut self, rank: u8) -> Self {
        self.rank = rank;
        self
    }

    #[must_use]
    pub fn tier(mut self, tier: impl Into<Tier>) -> Self {
        self.tier = tier.into();
        self
    }

    #[must_use]
    pub fn notes(mut self, notes: String) -> Self {
        self.notes = Some(notes);
        self
    }

    #[must_use]
    pub fn from_row(name: &str, header: &RowData, row: RowData) -> Option<Self> {
        let mut data = header
            .values
            .iter()
            .zip(row.values)
            .map(|(h, r)| {
                (h.formatted_value.as_deref().unwrap_or_default().to_lowercase(), r)
            })
            .collect::<HashMap<String, CellData>>();

        let weapon_name = data
            .remove("name")
            .ok_or(())
            .expect("data invariant")
            .formatted_value
            .ok_or(())
            .expect("data invariant");

        if weapon_name == "Ideal" {
            return None;
        }

        let reserves = data
            .remove("reserves")
            .map(|r| r.formatted_value.expect("data invariant"))
            .filter(|s| !matches!(s.as_str(), "?" | "N/A" | "TBA"))
            .map(|s| {
                s.parse().map_or_else(
                    |_| {
                        error!("temp"); // "Failed to parse: '{s}'"
                        None
                    },
                    Some,
                )
            })
            .ok_or(())
            .expect("Failed to read reserves");

        let shield = data
            .remove("shield")
            .map(|r| r.formatted_value.expect("data invariant"))
            .filter(|s| s != "?")
            .map(|s| s.parse().expect("data invariant"));
        let archetype = match name {
            "BGLs" | "HGLs" => String::from("Grenade Launcher"),
            "LMGs" => String::from("Machine Gun"),
            "LFRs" => String::from("Linear Fusion"),
            "HCs" => String::from("Hand Cannon"),
            "Other" => String::from("Other"),
            s => String::from(s.get(..s.len() - 1).ok_or(()).expect("temp")),
        };

        let mut weapon = Self::new(&weapon_name, archetype)
            .affinity(
                data.remove("affinity")
                    .or_else(|| data.remove("energy"))
                    .ok_or(())
                    .expect("affinity or energy should exist on data")
                    .formatted_value
                    .unwrap_or_default(),
            )
            .frame(
                data.remove("frame")
                    .map(|f| f.formatted_value.expect("data invariant")),
            )
            .enhanceable(
                data.remove("enhance")
                    .or_else(|| data.remove("⬆\u{fe0f}"))
                    .ok_or(())
                    .expect("enhance should exist on data")
                    .formatted_value
                    .ok_or(())
                    .expect("data invariant")
                    == "Yes",
            )
            .shield(shield)
            .reserves(reserves)
            .barrel(
                data.remove("barrel")
                    .ok_or(())
                    .expect("'barrel' should exist on data")
                    .formatted_value
                    .unwrap_or_default(),
            )
            .magazine(
                data.remove("mag")
                    .ok_or(())
                    .expect("'mag' should exist on data")
                    .formatted_value
                    .unwrap_or_default(),
            )
            .perk_1(
                data.remove("column 1")
                    .ok_or(())
                    .unwrap_or_else(|()| {
                        data.remove("perk 1").expect(
                            "Data should contain either 'perk' or 'column' headers",
                        )
                    })
                    .formatted_value
                    .ok_or(())
                    .expect("data invariant"),
            )
            .perk_2(
                data.remove("column 2")
                    .unwrap_or_else(|| {
                        data.remove("perk 2").expect(
                            "Data should contain either 'perk' or 'column' headers",
                        )
                    })
                    .formatted_value
                    .ok_or(())
                    .expect("data invariant"),
            )
            .origin_trait(
                data.remove("origin trait")
                    .ok_or(())
                    .expect("data invariant")
                    .formatted_value
                    .ok_or(())
                    .expect("data invariant"),
            )
            .rank(
                data.remove("rank")
                    .ok_or(())
                    .expect("data invariant")
                    .formatted_value
                    .map(|s| s.parse().expect("data invariant"))
                    .unwrap_or_default(),
            )
            .tier(data.remove("tier").ok_or(()).expect("data invariant"));

        if let Some(notes) = data.remove("notes").and_then(|d| d.formatted_value) {
            weapon = weapon.notes(notes);
        }

        Some(weapon)
    }

    #[must_use]
    pub fn build(self, item: &DestinyInventoryItemDefinition) -> Weapon {
        let icon = item.display_properties.icon.clone();

        Weapon {
            icon,
            name: self.name,
            archetype: self.archetype,
            affinity: self.affinity.parse().expect("data invariant"),
            frame: self
                .frame
                .map(|f| f.parse().expect("Failed to parse weapon frame")),
            enhanceable: self.enhanceable,
            reserves: self.reserves,
            barrel: self.barrel,
            magazine: self.magazine,
            perk_1: self.perk_1,
            perk_2: self.perk_2,
            origin_trait: self.origin_trait,
            rank: self.rank,
            tier: self.tier,
            notes: self.notes,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Weapon {
    pub icon: Option<String>,
    pub name: String,
    pub archetype: String,
    pub affinity: Affinity,
    pub frame: Option<Frame>,
    pub enhanceable: bool,
    pub reserves: Option<u16>,
    pub barrel: String,
    pub magazine: String,
    pub perk_1: String,
    pub perk_2: String,
    pub origin_trait: String,
    pub rank: u8,
    pub tier: Tier,
    pub notes: Option<String>,
}

impl Weapon {
    #[must_use]
    pub fn icon(&self) -> Option<String> {
        self.icon.as_deref().map(|icon| format!("https://www.bungie.net{icon}"))
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub fn archetype(&self) -> &str {
        &self.archetype
    }

    #[must_use]
    pub fn perks(&self) -> Perks<'_> {
        let barrel = self.barrel.split('\n').collect();
        let mag = self.magazine.split('\n').collect();
        let perk_1 = self.perk_1.split('\n').collect::<Vec<_>>();
        let perk_2 = self.perk_2.split('\n').collect::<Vec<_>>();

        Perks([barrel, mag, perk_1, perk_2])
    }

    #[must_use]
    pub fn origin_trait(&self) -> &str {
        &self.origin_trait
    }

    #[must_use]
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
                    DestinyItemType::None
                        | DestinyItemType::Pattern
                        | DestinyItemType::Dummy
                )
            })
            .filter(|item| {
                item.display_properties.name.to_lowercase().starts_with(&name)
            })
            .filter(|item| {
                item.inventory
                    .tier_type_name
                    .as_ref()
                    .is_some_and(|tier| tier.eq_ignore_ascii_case("Legendary"))
            })
            .filter_map(|item| item.sockets.as_ref().map(|sockets| (item, sockets)))
            .map(|(item, sockets)| {
                let plug_items =
                    sockets.socket_entries.iter().map(|socket| {
                        match (
                            socket.randomized_plug_set_hash,
                            socket.reusable_plug_set_hash,
                            socket.reusable_plug_items.as_slice(),
                        ) {
                            (Some(hash), None, items) => {
                                let plug_set = plug_manifest
                                    .get(&hash.to_string())
                                    .expect("data invariant");

                                plug_set
                                    .reusable_plug_items
                                    .iter()
                                    .map(|item| item.plug_item_hash)
                                    .chain(
                                        items.iter().map(|item| item.plug_item_hash),
                                    )
                                    .collect::<Vec<_>>()
                            },
                            (None, Some(hash), items) => {
                                let plug_set = plug_manifest
                                    .get(&hash.to_string())
                                    .expect("data invariant");

                                plug_set
                                    .reusable_plug_items
                                    .iter()
                                    .map(|item| item.plug_item_hash)
                                    .chain(
                                        items.iter().map(|item| item.plug_item_hash),
                                    )
                                    .collect::<Vec<_>>()
                            },
                            (Some(random_hash), Some(reusable_hash), items) => {
                                let random_plug_set = plug_manifest
                                    .get(&random_hash.to_string())
                                    .expect("data invariant");
                                let reusable_hash = plug_manifest
                                    .get(&reusable_hash.to_string())
                                    .expect("data invariant");

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
                                    .chain(
                                        items.iter().map(|item| item.plug_item_hash),
                                    )
                                    .collect::<Vec<_>>()
                            },
                            (None, None, items) => items
                                .iter()
                                .map(|item| item.plug_item_hash)
                                .collect::<Vec<_>>(),
                        }
                    });

                (item, plug_items)
            })
            .peekable();

        assert!(weapons.peek().is_some(), "At least 1 weapon should match: {name}");

        let weapons = weapons
            .map(|(item, plug_items)| {
                let perks = plug_items
                    .map(|traits| {
                        traits
                            .iter()
                            .map(ToString::to_string)
                            .map(|hash| {
                                item_manifest.get(&hash).expect("data invariant")
                            })
                            .filter(|item| !item.display_properties.name.is_empty())
                            .filter(|perk_item| {
                                self.perks().0.iter().flatten().any(|perk| {
                                    perk_item
                                        .display_properties
                                        .name
                                        .eq_ignore_ascii_case(perk)
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
            .map(|(hash, perks)| ApiWeapon { hash, perks: ApiPerks(perks) })
            .collect::<Vec<_>>();

        assert!(!weapons.is_empty(), "No weapon found for {name}");

        weapons
    }

    #[must_use]
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
        let frame =
            value.frame.as_ref().map(|f| format!("{f} ")).unwrap_or_default();

        let mut description =
            format!("Tier: {} (#{})", value.tier.tier(), value.rank);
        if let Some(reserves) = value.reserves {
            let _ = write!(description, "\nReserves: {reserves}");
        }

        let mut embed = CreateEmbed::new()
            .author(CreateEmbedAuthor::new(format!(
                "{} {}{}",
                value.affinity,
                frame,
                value.archetype(),
            )))
            .title(value.name.clone())
            .footer(CreateEmbedFooter::new("From 'Destiny 2: Endgame Analysis'"))
            .colour(value.tier.colour)
            .description(description)
            .fields(value.perks().iter().enumerate().flat_map(|(i, p)| {
                let title = match i {
                    0 => "Barrel",
                    1 => "Mag",
                    2 => "Perk 1",
                    3 => "Perk 2",
                    _ => "Perk", // Fallback for unexpected indices
                };

                let content = p
                    .iter()
                    .enumerate()
                    .map(|(j, line)| format!("{}. {}", j + 1, line))
                    .collect::<Vec<_>>()
                    .join("\n");

                let field = (title, content, true);

                if (i + 1) % 2 == 0 {
                    vec![
                        field,
                        // Name/Value: Zero Width Space
                        ("\u{200b}", "\u{200b}".to_string(), true),
                    ]
                } else {
                    vec![field]
                }
            }))
            .field("Origin Trait", value.origin_trait(), false);

        if let Some(icon) = value.icon() {
            embed = embed.thumbnail(icon);
        }

        if let Some(notes) = value.notes.as_deref() {
            let mut chars = notes.chars();
            let first_char = chars.next().expect("data invariant");
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

pub struct Perks<'a>([Vec<&'a str>; 4]);

impl<'a> Deref for Perks<'a> {
    type Target = [Vec<&'a str>; 4];

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
    #[must_use]
    pub fn as_wishlist(&self, item_hash: u32) -> String {
        create_combinations_string(&self.0, item_hash)
    }
}

#[must_use]
pub fn create_combinations_string(data: &[Vec<u32>], item_hash: u32) -> String {
    if data.is_empty() {
        return String::new();
    }

    let mut result = String::new();
    let mut current_combination = Vec::with_capacity(data.len());

    generate_combinations_iterative(
        data,
        0,
        &mut current_combination,
        &mut result,
        item_hash,
    );

    result
}

#[expect(
    clippy::indexing_slicing,
    reason = "data[depth] access is bounded: depth is incremented up to data.len() only"
)]
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
        write!(output, "dimwishlist:item={item_hash}&perks=")
            .expect("data invariant");
        for (i, &num) in current_combination.iter().enumerate() {
            if i > 0 {
                output.push(',');
            }
            write!(output, "{num}").expect("data invariant");
        }
        return;
    }

    for &num in data.get(depth).expect("temp") {
        current_combination.push(num);
        generate_combinations_iterative(
            data,
            depth + 1,
            current_combination,
            output,
            item_hash,
        );
        current_combination.pop();
    }
}
