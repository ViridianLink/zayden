use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::ops::Deref;

use bungie_api::types::destiny::DestinyItemType;
use bungie_api::{DestinyInventoryItemDefinition, DestinyPlugSetDefinition};
use google_sheets_api::types::sheet::{CellData, RowData};
use serenity::all::{
    AutocompleteChoice,
    CreateEmbed,
    CreateEmbedAuthor,
    CreateEmbedFooter,
};
use tracing::error;
use zayden_core::CoreError as ZaydenError;

use super::{Affinity, Frame, Tier};
use crate::endgame_analysis::EndgameAnalysisError;

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

    pub fn from_row(
        name: &str,
        header: &RowData,
        row: RowData,
    ) -> Result<Option<Self>, EndgameAnalysisError> {
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
            .ok_or_else(|| ZaydenError::missing_data("name column"))?
            .formatted_value
            .ok_or_else(|| ZaydenError::missing_data("name cell value"))?;

        if weapon_name == "Ideal" {
            return Ok(None);
        }

        let reserves = data
            .remove("reserves")
            .and_then(|r| r.formatted_value)
            .filter(|s| !matches!(s.as_str(), "?" | "N/A" | "TBA"))
            .and_then(|s| {
                s.parse().map_or_else(
                    |_| {
                        error!("Failed to parse reserves: {s:?}");
                        None
                    },
                    Some,
                )
            });

        let shield = data
            .remove("shield")
            .and_then(|r| r.formatted_value)
            .filter(|s| s != "?")
            .and_then(|s| s.parse().ok());

        let archetype = match name {
            "BGLs" | "HGLs" => String::from("Grenade Launcher"),
            "LMGs" => String::from("Machine Gun"),
            "LFRs" => String::from("Linear Fusion"),
            "HCs" => String::from("Hand Cannon"),
            "Other" => String::from("Other"),
            s => s
                .get(..s.len().saturating_sub(1))
                .ok_or_else(|| {
                    ZaydenError::missing_data("weapon type abbreviation".to_owned())
                })?
                .to_owned(),
        };

        let affinity = data
            .remove("affinity")
            .or_else(|| data.remove("energy"))
            .ok_or_else(|| ZaydenError::missing_data("affinity/energy column"))?
            .formatted_value
            .unwrap_or_default();

        let frame = data.remove("frame").and_then(|f| f.formatted_value);

        let enhance_cell = data
            .remove("enhance")
            .or_else(|| data.remove("⬆\u{fe0f}"))
            .ok_or_else(|| ZaydenError::missing_data("enhance column"))?;
        let enhanceable = enhance_cell.formatted_value.unwrap_or_default() == "Yes";

        let barrel = data
            .remove("barrel")
            .ok_or_else(|| ZaydenError::missing_data("barrel column"))?
            .formatted_value
            .unwrap_or_default();

        let magazine = data
            .remove("mag")
            .ok_or_else(|| ZaydenError::missing_data("mag column"))?
            .formatted_value
            .unwrap_or_default();

        let perk_1 = data
            .remove("column 1")
            .or_else(|| data.remove("perk 1"))
            .ok_or_else(|| ZaydenError::missing_data("perk 1 / column 1"))?
            .formatted_value
            .ok_or_else(|| ZaydenError::missing_data("perk 1 cell value"))?;

        let perk_2 = data
            .remove("column 2")
            .or_else(|| data.remove("perk 2"))
            .ok_or_else(|| ZaydenError::missing_data("perk 2 / column 2"))?
            .formatted_value
            .ok_or_else(|| ZaydenError::missing_data("perk 2 cell value"))?;

        let origin_trait = data
            .remove("origin trait")
            .ok_or_else(|| ZaydenError::missing_data("origin trait column"))?
            .formatted_value
            .ok_or_else(|| ZaydenError::missing_data("origin trait cell value"))?;

        let rank = data
            .remove("rank")
            .and_then(|c| c.formatted_value)
            .and_then(|s| s.parse().ok())
            .unwrap_or_default();

        let tier: Tier = data
            .remove("tier")
            .ok_or_else(|| ZaydenError::missing_data("tier column"))?
            .try_into()?;

        let mut weapon = Self::new(&weapon_name, archetype)
            .affinity(affinity)
            .frame(frame)
            .enhanceable(enhanceable)
            .shield(shield)
            .reserves(reserves)
            .barrel(barrel)
            .magazine(magazine)
            .perk_1(perk_1)
            .perk_2(perk_2)
            .origin_trait(origin_trait)
            .rank(rank)
            .tier(tier);

        if let Some(notes) = data.remove("notes").and_then(|d| d.formatted_value) {
            weapon = weapon.notes(notes);
        }

        Ok(Some(weapon))
    }

    pub fn build(
        self,
        item: &DestinyInventoryItemDefinition,
    ) -> Result<Weapon, EndgameAnalysisError> {
        let icon = item.display_properties.icon.clone();
        let affinity = if self.affinity.is_empty() {
            None
        } else {
            Some(
                self.affinity
                    .parse()
                    .map_err(|()| ZaydenError::missing_data("affinity parse"))?,
            )
        };
        let frame = self
            .frame
            .map(|f| {
                f.parse::<Frame>()
                    .map(|frame| frame.to_string())
                    .map_err(|()| ZaydenError::missing_data("frame parse"))
            })
            .transpose()?;

        Ok(Weapon {
            icon,
            name: self.name,
            archetype: self.archetype,
            affinity,
            frame,
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
        })
    }
}

#[derive(Debug)]
pub struct Weapon {
    pub icon: Option<String>,
    pub name: String,
    pub archetype: String,
    pub affinity: Option<Affinity>,
    pub frame: Option<String>,
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
                                let reusable = plug_manifest
                                    .get(&hash.to_string())
                                    .map(|s| {
                                        s.reusable_plug_items
                                            .iter()
                                            .map(|i| i.plug_item_hash)
                                            .collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default();
                                reusable
                                    .into_iter()
                                    .chain(
                                        items.iter().map(|item| item.plug_item_hash),
                                    )
                                    .collect::<Vec<_>>()
                            },
                            (None, Some(hash), items) => {
                                let reusable = plug_manifest
                                    .get(&hash.to_string())
                                    .map(|s| {
                                        s.reusable_plug_items
                                            .iter()
                                            .map(|i| i.plug_item_hash)
                                            .collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default();
                                reusable
                                    .into_iter()
                                    .chain(
                                        items.iter().map(|item| item.plug_item_hash),
                                    )
                                    .collect::<Vec<_>>()
                            },
                            (Some(random_hash), Some(reusable_hash), items) => {
                                let random_hashes = plug_manifest
                                    .get(&random_hash.to_string())
                                    .map(|s| {
                                        s.reusable_plug_items
                                            .iter()
                                            .map(|i| i.plug_item_hash)
                                            .collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default();
                                let reusable_hashes = plug_manifest
                                    .get(&reusable_hash.to_string())
                                    .map(|s| {
                                        s.reusable_plug_items
                                            .iter()
                                            .map(|i| i.plug_item_hash)
                                            .collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default();
                                random_hashes
                                    .into_iter()
                                    .chain(reusable_hashes)
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
                            .filter_map(|hash| item_manifest.get(&hash))
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

        let affinity =
            value.affinity.as_ref().map(ToString::to_string).unwrap_or_default();

        let mut description =
            format!("Tier: {} (#{})", value.tier.tier(), value.rank);
        if let Some(reserves) = value.reserves {
            let _ = write!(description, "\nReserves: {reserves}");
        }

        let mut embed = CreateEmbed::new()
            .author(CreateEmbedAuthor::new(format!(
                "{affinity} {frame}{}",
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
            embed = embed.thumbnail(icon, None);
        }

        if let Some(notes) = value.notes.as_deref() {
            let mut chars = notes.chars();
            if let Some(first_char) = chars.next() {
                let notes = first_char.to_uppercase().to_string() + chars.as_str();
                embed = embed.field("Notes", notes, false);
            }
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
        let _ = write!(output, "dimwishlist:item={item_hash}&perks=");
        let perks = current_combination
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        output.push_str(&perks);
        return;
    }

    if let Some(items) = data.get(depth) {
        for &num in items {
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
}
