use std::collections::HashSet;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;

use super::budget;
use super::domain::{Archetype, ArmourSlot, Class, Element, StatKind};
use super::mode::Mode;
use super::record::LoadoutRecord;
use crate::endgame_analysis::sheet::Affinity;
use crate::error::DraftError;

pub mod limits {
    pub const NAME: usize = 100;
    pub const TEXT: usize = 2000;
    pub const URL: usize = 512;
    pub const TAG: usize = 80;
    pub const TAGS: usize = 3;
    pub const ASPECTS: usize = 2;
    pub const FRAGMENTS: usize = 6;
    pub const WEAPONS: usize = 3;
    pub const PERKS: usize = 5;
    pub const MODS: usize = 5;
    pub const ARTIFACT_PERKS: usize = 12;
    pub const STAT_MAX: i16 = 200;
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EmojiKey(String);

impl EmojiKey {
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for EmojiKey {
    type Err = DraftError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let valid = (2..=32).contains(&s.len())
            && s.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_');
        if valid {
            Ok(Self(s.to_owned()))
        } else {
            Err(DraftError::InvalidEmojiKey(s.to_owned()))
        }
    }
}

impl Display for EmojiKey {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawAspect {
    pub aspect: String,
    pub fragments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawWeapon {
    pub name: String,
    pub affinity: Affinity,
    pub archetype: Archetype,
    pub icon_url: String,
    pub perks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawArmour {
    pub slot: ArmourSlot,
    pub name: String,
    pub icon_url: String,
    pub mods: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawLoadout {
    pub name: String,
    pub class: Class,
    pub element: Element,
    pub mode: Mode,
    pub tags: Vec<String>,
    pub super_name: String,
    pub super_emoji: String,
    pub class_ability: String,
    pub jump: String,
    pub melee: String,
    pub grenade: String,
    pub aspects: Vec<RawAspect>,
    pub weapons: Vec<RawWeapon>,
    pub armour: Vec<RawArmour>,
    pub stats: Vec<(StatKind, i16)>,
    pub artifact_name: String,
    pub artifact_perks: Vec<String>,
    pub author: String,
    pub dim_link: String,
    pub video_url: String,
    pub how_it_works: String,
}

#[derive(Debug, Clone)]
pub(crate) struct DraftAspect {
    pub(crate) aspect: EmojiKey,
    pub(crate) fragments: Vec<EmojiKey>,
}

#[derive(Debug, Clone)]
pub(crate) struct DraftWeapon {
    pub(crate) name: String,
    pub(crate) affinity: Affinity,
    pub(crate) archetype: Archetype,
    pub(crate) icon_url: String,
    pub(crate) perks: Vec<EmojiKey>,
}

#[derive(Debug, Clone)]
pub(crate) struct DraftArmour {
    pub(crate) slot: ArmourSlot,
    pub(crate) name: String,
    pub(crate) icon_url: String,
    pub(crate) mods: Vec<EmojiKey>,
}

#[derive(Debug, Clone)]
pub struct LoadoutDraft {
    pub(crate) name: String,
    pub(crate) class: Class,
    pub(crate) element: Element,
    pub(crate) mode: Mode,
    pub(crate) tags: Vec<String>,
    pub(crate) super_name: String,
    pub(crate) super_emoji: EmojiKey,
    pub(crate) class_ability: EmojiKey,
    pub(crate) jump: EmojiKey,
    pub(crate) melee: EmojiKey,
    pub(crate) grenade: EmojiKey,
    pub(crate) aspects: Vec<DraftAspect>,
    pub(crate) weapons: Vec<DraftWeapon>,
    pub(crate) armour: Vec<DraftArmour>,
    pub(crate) stats: Vec<(StatKind, i16)>,
    pub(crate) artifact_name: Option<String>,
    pub(crate) artifact_perks: Vec<EmojiKey>,
    pub(crate) author: String,
    pub(crate) dim_link: String,
    pub(crate) video_url: Option<String>,
    pub(crate) how_it_works: Option<String>,
}

impl TryFrom<RawLoadout> for LoadoutDraft {
    type Error = DraftError;

    fn try_from(raw: RawLoadout) -> Result<Self, Self::Error> {
        let aspects = raw
            .aspects
            .iter()
            .filter(|a| !a.aspect.trim().is_empty())
            .map(|a| {
                Ok(DraftAspect {
                    aspect: key("aspect", &a.aspect)?,
                    fragments: keys("fragments", &a.fragments, limits::FRAGMENTS)?,
                })
            })
            .collect::<Result<Vec<_>, DraftError>>()?;

        let weapons = raw
            .weapons
            .iter()
            .filter(|w| !w.name.trim().is_empty())
            .map(|w| {
                Ok(DraftWeapon {
                    name: required("weapon name", &w.name, limits::NAME)?,
                    affinity: w.affinity,
                    archetype: w.archetype,
                    icon_url: https("weapon icon", &w.icon_url)?
                        .ok_or(DraftError::Required { field: "weapon icon" })?,
                    perks: keys("perks", &w.perks, limits::PERKS)?,
                })
            })
            .collect::<Result<Vec<_>, DraftError>>()?;

        let mut slots = HashSet::new();
        let armour = raw
            .armour
            .iter()
            .filter(|a| !a.name.trim().is_empty())
            .map(|a| {
                if !slots.insert(a.slot) {
                    return Err(DraftError::DuplicateArmourSlot(a.slot));
                }
                Ok(DraftArmour {
                    slot: a.slot,
                    name: required("armour name", &a.name, limits::NAME)?,
                    icon_url: https("armour icon", &a.icon_url)?
                        .ok_or(DraftError::Required { field: "armour icon" })?,
                    mods: keys("mods", &a.mods, limits::MODS)?,
                })
            })
            .collect::<Result<Vec<_>, DraftError>>()?;

        let mut seen = HashSet::new();
        for &(stat, value) in &raw.stats {
            if !seen.insert(stat) {
                return Err(DraftError::DuplicateStat(stat));
            }
            if !(0..=limits::STAT_MAX).contains(&value) {
                return Err(DraftError::StatOutOfRange {
                    stat,
                    value,
                    max: limits::STAT_MAX,
                });
            }
        }

        let draft = Self {
            name: required("name", &raw.name, limits::NAME)?,
            tags: tags(&raw.tags, raw.element, raw.mode)?,
            super_name: required("super name", &raw.super_name, limits::NAME)?,
            super_emoji: key("super emoji", &raw.super_emoji)?,
            class_ability: key("class ability", &raw.class_ability)?,
            jump: key("jump", &raw.jump)?,
            melee: key("melee", &raw.melee)?,
            grenade: key("grenade", &raw.grenade)?,
            aspects: capped("aspects", aspects, limits::ASPECTS)?,
            weapons: capped("weapons", weapons, limits::WEAPONS)?,
            armour,
            stats: raw.stats,
            artifact_name: optional(
                "artifact name",
                &raw.artifact_name,
                limits::NAME,
            )?,
            artifact_perks: keys(
                "artifact perks",
                &raw.artifact_perks,
                limits::ARTIFACT_PERKS,
            )?,
            author: required("author", &raw.author, limits::NAME)?,
            dim_link: https("DIM link", &raw.dim_link)?
                .ok_or(DraftError::Required { field: "DIM link" })?,
            video_url: https("video URL", &raw.video_url)?,
            how_it_works: optional("how it works", &raw.how_it_works, limits::TEXT)?,
            class: raw.class,
            element: raw.element,
            mode: raw.mode,
        };
        within_discord_limits(&draft)?;
        Ok(draft)
    }
}

fn within_discord_limits(draft: &LoadoutDraft) -> Result<(), DraftError> {
    let needed = budget::components(
        draft.tags.len(),
        draft.weapons.len(),
        draft.armour.len(),
    );
    if needed > budget::MAX_COMPONENTS {
        return Err(DraftError::TooManyComponents {
            needed,
            max: budget::MAX_COMPONENTS,
        });
    }
    let estimate = budget::text(draft);
    if estimate > budget::MAX_TEXT {
        return Err(DraftError::TooMuchText { estimate, max: budget::MAX_TEXT });
    }
    Ok(())
}

fn optional(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<Option<String>, DraftError> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.chars().count() > max {
        return Err(DraftError::TooLong { field, max });
    }
    Ok(Some(value.to_owned()))
}

fn required(
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<String, DraftError> {
    optional(field, value, max)?.ok_or(DraftError::Required { field })
}

fn https(field: &'static str, value: &str) -> Result<Option<String>, DraftError> {
    match optional(field, value, limits::URL)? {
        Some(url) if !url.starts_with("https://") => {
            Err(DraftError::NotHttps { field })
        },
        url => Ok(url),
    }
}

fn key(field: &'static str, value: &str) -> Result<EmojiKey, DraftError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(DraftError::Required { field });
    }
    value.parse()
}

fn keys(
    field: &'static str,
    values: &[String],
    max: usize,
) -> Result<Vec<EmojiKey>, DraftError> {
    let keys = values
        .iter()
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
        .map(str::parse)
        .collect::<Result<Vec<EmojiKey>, _>>()?;
    capped(field, keys, max)
}

fn capped<T>(
    field: &'static str,
    items: Vec<T>,
    max: usize,
) -> Result<Vec<T>, DraftError> {
    if items.len() > max {
        Err(DraftError::TooMany { field, max })
    } else {
        Ok(items)
    }
}

fn tags(
    values: &[String],
    element: Element,
    mode: Mode,
) -> Result<Vec<String>, DraftError> {
    let mut taken = HashSet::from([element.key(), mode.to_string()]);
    let mut tags = Vec::new();
    for tag in values.iter().map(|t| t.trim()).filter(|t| !t.is_empty()) {
        if tag.chars().count() > limits::TAG {
            return Err(DraftError::TooLong { field: "tag", max: limits::TAG });
        }
        if !taken.insert(tag.to_owned()) {
            return Err(DraftError::DuplicateTag(tag.to_owned()));
        }
        tags.push(tag.to_owned());
    }
    capped("tags", tags, limits::TAGS)
}

impl From<LoadoutRecord> for RawLoadout {
    fn from(r: LoadoutRecord) -> Self {
        Self {
            name: r.name,
            class: r.class,
            element: r.element,
            mode: r.mode,
            tags: r.tags,
            super_name: r.super_name,
            super_emoji: r.super_emoji,
            class_ability: r.class_ability,
            jump: r.jump,
            melee: r.melee,
            grenade: r.grenade,
            aspects: r
                .aspects
                .into_iter()
                .map(|a| RawAspect { aspect: a.emoji, fragments: a.fragments })
                .collect(),
            weapons: r
                .weapons
                .into_iter()
                .map(|w| RawWeapon {
                    name: w.name,
                    affinity: w.affinity,
                    archetype: w.archetype,
                    icon_url: w.icon_url,
                    perks: w.perks,
                })
                .collect(),
            armour: r
                .armour
                .into_iter()
                .map(|a| RawArmour {
                    slot: a.slot,
                    name: a.name,
                    icon_url: a.icon_url,
                    mods: a.mods,
                })
                .collect(),
            stats: r.stats,
            artifact_name: r.artifact_name.unwrap_or_default(),
            artifact_perks: r.artifact_perks,
            author: r.author,
            dim_link: r.dim_link,
            video_url: r.video_url.unwrap_or_default(),
            how_it_works: r.how_it_works.unwrap_or_default(),
        }
    }
}
