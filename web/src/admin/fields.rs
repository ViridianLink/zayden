use std::collections::{BTreeMap, HashSet};

use super::dto::{ArmourForm, AspectForm, LoadoutForm, StatForm, WeaponForm};
use super::error::LoadoutFieldError;

#[must_use]
pub fn row_name(list: &str, index: usize, field: &str) -> String {
    format!("{list}[{index}].{field}")
}

fn split_row(key: &str) -> Option<(&str, usize, &str)> {
    let (list, rest) = key.split_once('[')?;
    let (index, field) = rest.split_once("].")?;
    let canonical = index.bytes().all(|b| b.is_ascii_digit())
        && (index == "0" || !index.starts_with('0'));
    if !canonical {
        return None;
    }
    Some((list, index.parse().ok()?, field))
}

#[derive(Default)]
struct Fold {
    form: LoadoutForm,
    seen: HashSet<String>,
    aspects: BTreeMap<usize, AspectForm>,
    weapons: BTreeMap<usize, WeaponForm>,
    armour: BTreeMap<usize, ArmourForm>,
    stats: BTreeMap<usize, StatForm>,
}

fn once(
    seen: &mut HashSet<String>,
    slot: &mut String,
    key: String,
    value: String,
) -> Result<(), LoadoutFieldError> {
    if seen.contains(&key) {
        return Err(LoadoutFieldError::RepeatedField(key));
    }
    seen.insert(key);
    *slot = value;
    Ok(())
}

impl Fold {
    fn top(&mut self, key: String, value: String) -> Result<(), LoadoutFieldError> {
        let form = &mut self.form;
        let slot = match key.as_str() {
            "id" => return self.id(key, value),
            "tags" => {
                form.tags.push(value);
                return Ok(());
            },
            "artifact_perks" => {
                form.artifact_perks.push(value);
                return Ok(());
            },
            "name" => &mut form.name,
            "class" => &mut form.class,
            "element" => &mut form.element,
            "mode" => &mut form.mode,
            "super_name" => &mut form.super_name,
            "super_emoji" => &mut form.super_emoji,
            "class_ability" => &mut form.class_ability,
            "jump" => &mut form.jump,
            "melee" => &mut form.melee,
            "grenade" => &mut form.grenade,
            "artifact_name" => &mut form.artifact_name,
            "author" => &mut form.author,
            "dim_link" => &mut form.dim_link,
            "video_url" => &mut form.video_url,
            "how_it_works" => &mut form.how_it_works,
            _ => return Err(LoadoutFieldError::UnknownField(key)),
        };
        once(&mut self.seen, slot, key, value)
    }

    fn id(&mut self, key: String, value: String) -> Result<(), LoadoutFieldError> {
        if !self.seen.insert(key.clone()) {
            return Err(LoadoutFieldError::RepeatedField(key));
        }
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            self.form.id =
                Some(trimmed.parse().map_err(|_parse_int_error| {
                    LoadoutFieldError::InvalidId(value)
                })?);
        }
        Ok(())
    }

    fn row(
        &mut self,
        key: String,
        list: &str,
        index: usize,
        field: &str,
        value: String,
    ) -> Result<(), LoadoutFieldError> {
        let slot = match (list, field) {
            ("aspects", "fragments") => {
                self.aspects.entry(index).or_default().fragments.push(value);
                return Ok(());
            },
            ("weapons", "perks") => {
                self.weapons.entry(index).or_default().perks.push(value);
                return Ok(());
            },
            ("armour", "mods") => {
                self.armour.entry(index).or_default().mods.push(value);
                return Ok(());
            },
            ("aspects", "aspect") => {
                &mut self.aspects.entry(index).or_default().aspect
            },
            ("weapons", "name") => &mut self.weapons.entry(index).or_default().name,
            ("weapons", "affinity") => {
                &mut self.weapons.entry(index).or_default().affinity
            },
            ("weapons", "archetype") => {
                &mut self.weapons.entry(index).or_default().archetype
            },
            ("weapons", "icon_url") => {
                &mut self.weapons.entry(index).or_default().icon_url
            },
            ("armour", "slot") => &mut self.armour.entry(index).or_default().slot,
            ("armour", "name") => &mut self.armour.entry(index).or_default().name,
            ("armour", "icon_url") => {
                &mut self.armour.entry(index).or_default().icon_url
            },
            ("stats", "stat") => &mut self.stats.entry(index).or_default().stat,
            ("stats", "value") => &mut self.stats.entry(index).or_default().value,
            _ => return Err(LoadoutFieldError::UnknownField(key)),
        };
        once(&mut self.seen, slot, key, value)
    }

    fn finish(self) -> LoadoutForm {
        LoadoutForm {
            aspects: self.aspects.into_values().collect(),
            weapons: self.weapons.into_values().collect(),
            armour: self.armour.into_values().collect(),
            stats: self.stats.into_values().collect(),
            ..self.form
        }
    }
}

pub fn fold_loadout_form<I>(pairs: I) -> Result<LoadoutForm, LoadoutFieldError>
where
    I: IntoIterator<Item = (String, String)>,
{
    let mut fold = Fold::default();
    for (key, value) in pairs {
        match split_row(&key) {
            Some((list, index, field)) => {
                let (list, field) = (list.to_owned(), field.to_owned());
                fold.row(key, &list, index, &field, value)?;
            },
            None => fold.top(key, value)?,
        }
    }
    Ok(fold.finish())
}

impl TryFrom<Vec<(String, String)>> for LoadoutForm {
    type Error = LoadoutFieldError;

    fn try_from(pairs: Vec<(String, String)>) -> Result<Self, Self::Error> {
        fold_loadout_form(pairs)
    }
}

#[must_use]
pub fn loadout_pairs(form: &LoadoutForm) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    let mut push = |key: String, value: &str| pairs.push((key, value.to_owned()));

    if let Some(id) = form.id {
        push("id".to_owned(), &id.to_string());
    }
    for (key, value) in [
        ("name", &form.name),
        ("class", &form.class),
        ("element", &form.element),
        ("mode", &form.mode),
        ("super_name", &form.super_name),
        ("super_emoji", &form.super_emoji),
        ("class_ability", &form.class_ability),
        ("jump", &form.jump),
        ("melee", &form.melee),
        ("grenade", &form.grenade),
        ("artifact_name", &form.artifact_name),
        ("author", &form.author),
        ("dim_link", &form.dim_link),
        ("video_url", &form.video_url),
        ("how_it_works", &form.how_it_works),
    ] {
        push(key.to_owned(), value);
    }
    for tag in &form.tags {
        push("tags".to_owned(), tag);
    }
    for perk in &form.artifact_perks {
        push("artifact_perks".to_owned(), perk);
    }
    for (i, a) in form.aspects.iter().enumerate() {
        push(row_name("aspects", i, "aspect"), &a.aspect);
        for fragment in &a.fragments {
            push(row_name("aspects", i, "fragments"), fragment);
        }
    }
    for (i, w) in form.weapons.iter().enumerate() {
        push(row_name("weapons", i, "name"), &w.name);
        push(row_name("weapons", i, "affinity"), &w.affinity);
        push(row_name("weapons", i, "archetype"), &w.archetype);
        push(row_name("weapons", i, "icon_url"), &w.icon_url);
        for perk in &w.perks {
            push(row_name("weapons", i, "perks"), perk);
        }
    }
    for (i, a) in form.armour.iter().enumerate() {
        push(row_name("armour", i, "slot"), &a.slot);
        push(row_name("armour", i, "name"), &a.name);
        push(row_name("armour", i, "icon_url"), &a.icon_url);
        for m in &a.mods {
            push(row_name("armour", i, "mods"), m);
        }
    }
    for (i, s) in form.stats.iter().enumerate() {
        push(row_name("stats", i, "stat"), &s.stat);
        push(row_name("stats", i, "value"), &s.value);
    }
    pairs
}
