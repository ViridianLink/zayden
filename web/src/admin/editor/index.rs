use std::collections::{HashMap, HashSet};

use crate::admin::dto::{LoadoutCatalog, LoadoutForm};
use crate::admin::keys::{emoji_url, enum_key};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CatalogIndex {
    images: HashMap<String, String>,
    reserved: HashSet<String>,
}

impl CatalogIndex {
    #[must_use]
    pub fn new(c: &LoadoutCatalog) -> Self {
        let o = &c.options;
        let reserved = [
            &o.classes,
            &o.elements,
            &o.modes,
            &o.affinities,
            &o.archetypes,
            &o.stats,
            &o.armour_slots,
        ]
        .into_iter()
        .flatten()
        .map(|label| enum_key(label))
        .collect();
        let images =
            c.emojis.iter().map(|e| (e.name.clone(), emoji_url(&e.id))).collect();
        Self { images, reserved }
    }

    #[must_use]
    pub fn image(&self, key: &str) -> Option<&str> {
        self.images.get(key).map(String::as_str)
    }

    #[must_use]
    pub fn has_emoji(&self, key: &str) -> bool {
        self.images.contains_key(key)
    }

    #[must_use]
    pub fn has_any_emoji(&self) -> bool {
        !self.images.is_empty()
    }

    #[must_use]
    pub fn is_reserved(&self, key: &str) -> bool {
        self.reserved.contains(key)
    }

    #[must_use]
    pub fn enum_image(&self, label: &str) -> Option<&str> {
        self.image(&enum_key(label))
    }
}

#[must_use]
pub fn emoji_keys(form: &LoadoutForm) -> Vec<String> {
    let mut keys = vec![
        form.super_emoji.clone(),
        form.class_ability.clone(),
        form.jump.clone(),
        form.melee.clone(),
        form.grenade.clone(),
    ];
    for a in &form.aspects {
        keys.push(a.aspect.clone());
        keys.extend(a.fragments.iter().cloned());
    }
    for w in &form.weapons {
        keys.extend(w.perks.iter().cloned());
    }
    for a in &form.armour {
        keys.extend(a.mods.iter().cloned());
    }
    keys.extend(form.artifact_perks.iter().cloned());
    keys.retain(|k| !k.trim().is_empty());
    keys
}

#[must_use]
pub fn missing_emojis(index: &CatalogIndex, form: &LoadoutForm) -> Vec<String> {
    if !index.has_any_emoji() {
        return Vec::new();
    }
    let mut missing = emoji_keys(form);
    missing.retain(|k| !index.has_emoji(k));
    missing.sort();
    missing.dedup();
    missing
}

#[must_use]
pub fn draft_key(id: Option<i32>) -> String {
    id.map_or_else(
        || "zayden:loadout-draft:new".to_owned(),
        |id| format!("zayden:loadout-draft:{id}"),
    )
}

#[must_use]
pub fn editor_id(raw: &str) -> Option<i32> {
    raw.parse::<i32>().ok()
}
