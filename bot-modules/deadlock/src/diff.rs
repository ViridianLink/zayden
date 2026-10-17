use std::collections::{BTreeMap, BTreeSet};

use deadlock_api::model::{
    AbilityUpgrade,
    Hero,
    Item,
    ItemKind,
    ItemProperty,
    StatValue,
    WeaponInfo,
};
use serde::{Deserialize, Serialize};

const WEAPON_SLOT: &str = "weapon_primary";
const SIGNATURE_SLOTS: [&str; 4] =
    ["signature1", "signature2", "signature3", "signature4"];
const LEVEL_UP_PREFIX: &str = "MODIFIER_VALUE_";
const EPSILON: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Presence {
    Added,
    Removed,
    Changed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Change {
    pub label: String,
    pub old: Option<String>,
    pub new: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityDiff {
    pub item_id: u32,
    pub name: String,
    pub presence: Presence,
    pub changes: Vec<Change>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeroDiff {
    pub hero_id: u32,
    pub name: String,
    pub presence: Presence,
    pub stats: Vec<Change>,
    pub weapon: Vec<Change>,
    pub abilities: Vec<AbilityDiff>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemDiff {
    pub item_id: u32,
    pub name: String,
    pub presence: Presence,
    pub changes: Vec<Change>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PatchDiff {
    pub heroes: Vec<HeroDiff>,
    pub items: Vec<ItemDiff>,
}

impl PatchDiff {
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.heroes.is_empty() && self.items.is_empty()
    }

    #[must_use]
    pub fn hero_ids(&self) -> Vec<u32> {
        self.heroes.iter().map(|hero| hero.hero_id).collect()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Snapshot<'a> {
    pub heroes: &'a [Hero],
    pub items: &'a [Item],
}

#[must_use]
pub fn diff(old: &Snapshot<'_>, new: &Snapshot<'_>) -> PatchDiff {
    PatchDiff { heroes: diff_heroes(old, new), items: diff_items(old, new) }
}

const fn playable(hero: &Hero) -> bool {
    hero.player_selectable && !hero.disabled && !hero.in_development
}

const fn shop_item(item: &Item) -> bool {
    matches!(item.kind, ItemKind::Upgrade) && item.shopable
}

fn by_class<'a>(items: &'a [Item], class_name: &str) -> Option<&'a Item> {
    items.iter().find(|item| item.class_name == class_name)
}

fn humanize(key: &str) -> String {
    let spaced = key.replace('_', " ").to_lowercase();
    let mut chars = spaced.chars();

    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

fn numeric(value: &StatValue) -> Option<f64> {
    match value {
        StatValue::Number(n) => Some(*n),
        StatValue::Text(text) => text.trim().parse().ok(),
    }
}

fn same(old: Option<&StatValue>, new: Option<&StatValue>) -> bool {
    match (old, new) {
        (None, None) => true,
        (Some(old), Some(new)) => match (old.as_f64(), new.as_f64()) {
            (Some(old), Some(new)) => (old - new).abs() < EPSILON,
            _ => old.to_string() == new.to_string(),
        },
        _ => false,
    }
}

fn value_change(
    label: impl Into<String>,
    old: Option<&StatValue>,
    new: Option<&StatValue>,
) -> Option<Change> {
    (!same(old, new)).then(|| Change {
        label: label.into(),
        old: old.map(ToString::to_string),
        new: new.map(ToString::to_string),
    })
}

fn number_change(
    label: impl Into<String>,
    old: Option<f64>,
    new: Option<f64>,
) -> Option<Change> {
    value_change(
        label,
        old.map(StatValue::Number).as_ref(),
        new.map(StatValue::Number).as_ref(),
    )
}

fn union_keys<'a, V>(
    old: &'a BTreeMap<String, V>,
    new: &'a BTreeMap<String, V>,
) -> BTreeSet<&'a str> {
    old.keys().chain(new.keys()).map(String::as_str).collect()
}

fn hero_only(hero: &Hero, presence: Presence) -> HeroDiff {
    HeroDiff {
        hero_id: hero.id,
        name: hero.name.clone(),
        presence,
        stats: Vec::new(),
        weapon: Vec::new(),
        abilities: Vec::new(),
    }
}

fn diff_heroes(old: &Snapshot<'_>, new: &Snapshot<'_>) -> Vec<HeroDiff> {
    let old_by_id: BTreeMap<u32, &Hero> =
        old.heroes.iter().map(|hero| (hero.id, hero)).collect();
    let new_playable: BTreeSet<u32> = new
        .heroes
        .iter()
        .filter(|hero| playable(hero))
        .map(|hero| hero.id)
        .collect();

    let mut out = Vec::new();

    for hero in new.heroes.iter().filter(|hero| playable(hero)) {
        match old_by_id.get(&hero.id).filter(|before| playable(before)) {
            None => out.push(hero_only(hero, Presence::Added)),
            Some(before) => out.extend(changed_hero(before, hero, old, new)),
        }
    }

    for hero in old.heroes.iter().filter(|hero| playable(hero)) {
        if !new_playable.contains(&hero.id) {
            out.push(hero_only(hero, Presence::Removed));
        }
    }

    out.sort_by(|a, b| a.name.cmp(&b.name).then(a.hero_id.cmp(&b.hero_id)));
    out
}

fn changed_hero(
    before: &Hero,
    after: &Hero,
    old: &Snapshot<'_>,
    new: &Snapshot<'_>,
) -> Option<HeroDiff> {
    let stats = stat_changes(before, after);
    let weapon = weapon_changes(before, after, old.items, new.items);
    let abilities = ability_diffs(before, after, old.items, new.items);

    if stats.is_empty() && weapon.is_empty() && abilities.is_empty() {
        return None;
    }

    Some(HeroDiff {
        hero_id: after.id,
        name: after.name.clone(),
        presence: Presence::Changed,
        stats,
        weapon,
        abilities,
    })
}

fn stat_changes(before: &Hero, after: &Hero) -> Vec<Change> {
    let mut changes = Vec::new();

    for key in union_keys(&before.starting_stats, &after.starting_stats) {
        changes.extend(value_change(
            humanize(key),
            before.starting_stats.get(key).map(|stat| &stat.value),
            after.starting_stats.get(key).map(|stat| &stat.value),
        ));
    }

    for key in union_keys(
        &before.standard_level_up_upgrades,
        &after.standard_level_up_upgrades,
    ) {
        let name = key.strip_prefix(LEVEL_UP_PREFIX).unwrap_or(key);
        changes.extend(number_change(
            format!("{} per level", humanize(name)),
            before.standard_level_up_upgrades.get(key).copied(),
            after.standard_level_up_upgrades.get(key).copied(),
        ));
    }

    changes
}

fn weapon_info<'a>(hero: &Hero, items: &'a [Item]) -> Option<&'a WeaponInfo> {
    hero.items
        .get(WEAPON_SLOT)
        .and_then(|class_name| by_class(items, class_name))
        .map(|item| &item.weapon_info)
}

const fn weapon_fields(info: &WeaponInfo) -> [(&'static str, Option<f64>); 9] {
    [
        ("Bullet damage", info.bullet_damage),
        ("Bullets per shot", info.bullets),
        ("Magazine size", info.clip_size),
        ("Fire interval (s)", info.cycle_time),
        ("Reload time (s)", info.reload_duration),
        ("Bullet speed", info.bullet_speed),
        ("Crit multiplier", info.crit_bonus_start),
        ("Falloff start range", info.damage_falloff_start_range),
        ("Falloff end range", info.damage_falloff_end_range),
    ]
}

fn weapon_changes(
    before: &Hero,
    after: &Hero,
    old_items: &[Item],
    new_items: &[Item],
) -> Vec<Change> {
    let (Some(old), Some(new)) =
        (weapon_info(before, old_items), weapon_info(after, new_items))
    else {
        return Vec::new();
    };

    weapon_fields(old)
        .into_iter()
        .zip(weapon_fields(new))
        .filter_map(|((label, old), (_, new))| number_change(label, old, new))
        .collect()
}

fn ability_diffs(
    before: &Hero,
    after: &Hero,
    old_items: &[Item],
    new_items: &[Item],
) -> Vec<AbilityDiff> {
    let mut out = Vec::new();

    for slot in SIGNATURE_SLOTS {
        let old =
            before.items.get(slot).and_then(|class| by_class(old_items, class));
        let new = after.items.get(slot).and_then(|class| by_class(new_items, class));

        match (old, new) {
            (Some(old), Some(new)) if old.id == new.id => {
                let changes = ability_changes(old, new);
                if !changes.is_empty() {
                    out.push(ability_diff(new, Presence::Changed, changes));
                }
            },
            (old, new) => {
                out.extend(
                    new.map(|item| ability_diff(item, Presence::Added, Vec::new())),
                );
                out.extend(
                    old.map(|item| {
                        ability_diff(item, Presence::Removed, Vec::new())
                    }),
                );
            },
        }
    }

    out
}

fn ability_diff(
    item: &Item,
    presence: Presence,
    changes: Vec<Change>,
) -> AbilityDiff {
    AbilityDiff { item_id: item.id, name: item.name.clone(), presence, changes }
}

fn ability_changes(old: &Item, new: &Item) -> Vec<Change> {
    let mut changes = property_changes(old, new);
    changes.extend(upgrade_changes(old, new));
    changes
}

fn label_of(property: &ItemProperty) -> Option<&str> {
    property.label.as_deref().filter(|label| !label.is_empty())
}

fn format_property(property: &ItemProperty) -> Option<String> {
    let value = property.value.as_ref()?;

    let prefix = property.prefix.as_deref().unwrap_or_default();
    let postfix = property
        .postfix
        .as_deref()
        .filter(|_| numeric(value).is_some())
        .unwrap_or_default();

    Some(format!("{prefix}{value}{postfix}"))
}

fn property_changes(old: &Item, new: &Item) -> Vec<Change> {
    union_keys(&old.properties, &new.properties)
        .into_iter()
        .filter_map(|key| {
            let before = old.properties.get(key);
            let after = new.properties.get(key);
            let label =
                after.and_then(label_of).or_else(|| before.and_then(label_of))?;

            let unchanged = same(
                before.and_then(|property| property.value.as_ref()),
                after.and_then(|property| property.value.as_ref()),
            );

            (!unchanged).then(|| Change {
                label: label.to_owned(),
                old: before.and_then(format_property),
                new: after.and_then(format_property),
            })
        })
        .collect()
}

fn bonus_text(bonus: &StatValue) -> String {
    numeric(bonus).map_or_else(|| bonus.to_string(), |n| format!("{n:+}"))
}

fn tier_text(upgrade: &AbilityUpgrade, item: &Item) -> Option<String> {
    let text = upgrade
        .property_upgrades
        .iter()
        .map(|change| {
            let name = item
                .properties
                .get(&change.name)
                .and_then(label_of)
                .unwrap_or(change.name.as_str());

            change.bonus.as_ref().map_or_else(
                || name.to_owned(),
                |bonus| format!("{name} {}", bonus_text(bonus)),
            )
        })
        .collect::<Vec<_>>()
        .join(", ");

    (!text.is_empty()).then_some(text)
}

fn upgrade_changes(old: &Item, new: &Item) -> Vec<Change> {
    (0..old.upgrades.len().max(new.upgrades.len()))
        .filter_map(|tier| {
            let before =
                old.upgrades.get(tier).and_then(|upgrade| tier_text(upgrade, old));
            let after =
                new.upgrades.get(tier).and_then(|upgrade| tier_text(upgrade, new));

            (before != after).then(|| Change {
                label: format!("T{} upgrade", tier + 1),
                old: before,
                new: after,
            })
        })
        .collect()
}

fn item_only(item: &Item, presence: Presence) -> ItemDiff {
    ItemDiff {
        item_id: item.id,
        name: item.name.clone(),
        presence,
        changes: Vec::new(),
    }
}

fn item_changes(before: &Item, after: &Item) -> Vec<Change> {
    let mut changes = Vec::new();

    changes.extend(number_change(
        "Cost",
        before.cost.map(f64::from),
        after.cost.map(f64::from),
    ));
    changes.extend(number_change(
        "Tier",
        before.item_tier.map(f64::from),
        after.item_tier.map(f64::from),
    ));
    changes.extend(property_changes(before, after));

    changes
}

fn diff_items(old: &Snapshot<'_>, new: &Snapshot<'_>) -> Vec<ItemDiff> {
    let old_shop: BTreeMap<u32, &Item> = old
        .items
        .iter()
        .filter(|item| shop_item(item))
        .map(|item| (item.id, item))
        .collect();
    let new_shop: BTreeMap<u32, &Item> = new
        .items
        .iter()
        .filter(|item| shop_item(item))
        .map(|item| (item.id, item))
        .collect();

    let mut out = Vec::new();

    for (id, item) in &new_shop {
        match old_shop.get(id) {
            None => out.push(item_only(item, Presence::Added)),
            Some(before) => {
                let changes = item_changes(before, item);
                if !changes.is_empty() {
                    out.push(ItemDiff {
                        item_id: item.id,
                        name: item.name.clone(),
                        presence: Presence::Changed,
                        changes,
                    });
                }
            },
        }
    }

    for (id, item) in &old_shop {
        if !new_shop.contains_key(id) {
            out.push(item_only(item, Presence::Removed));
        }
    }

    out.sort_by(|a, b| a.name.cmp(&b.name).then(a.item_id.cmp(&b.item_id)));
    out
}
