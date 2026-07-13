use std::collections::HashMap;
use std::hash::Hash;

use crate::model::{Item, Pal, PassiveSkill};
use crate::source::{Category, SourceId};

#[must_use]
pub fn nonempty(value: Option<String>) -> Option<String> {
    value.map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
}

pub fn dedup<T, K, F>(items: impl IntoIterator<Item = T>, key: F) -> Vec<T>
where
    K: PartialEq,
    F: Fn(&T) -> K,
{
    let mut out: Vec<T> = Vec::new();
    for item in items {
        if !out.iter().any(|existing| key(existing) == key(&item)) {
            out.push(item);
        }
    }
    out
}

#[must_use]
pub fn consensus<T>(
    field: &str,
    category: Category,
    candidates: &[(SourceId, Option<T>)],
) -> Option<T>
where
    T: Eq + Hash + Clone,
{
    let mut tally: HashMap<&T, (usize, usize)> = HashMap::new();
    for (source, value) in candidates {
        let Some(value) = value else { continue };
        let rank = category.rank(*source);
        let entry = tally.entry(value).or_insert((0, usize::MAX));
        entry.0 += 1;
        entry.1 = entry.1.min(rank);
    }

    if tally.len() > 1 {
        tracing::debug!(field, ?category, votes = tally.len(), "source conflict");
    }

    tally
        .iter()
        .max_by(|a, b| a.1.0.cmp(&b.1.0).then_with(|| b.1.1.cmp(&a.1.1)))
        .map(|(value, _)| (*value).clone())
}

fn collect<T, F>(candidates: &[(SourceId, Pal)], f: F) -> Vec<(SourceId, Option<T>)>
where
    F: Fn(&Pal) -> Option<T>,
{
    candidates.iter().map(|(source, pal)| (*source, f(pal))).collect()
}

fn text<F>(
    candidates: &[(SourceId, Pal)],
    field: &str,
    cat: Category,
    f: F,
) -> Option<String>
where
    F: Fn(&Pal) -> Option<String>,
{
    consensus(field, cat, &collect(candidates, |p| nonempty(f(p))))
}

fn union_by_precedence<T, K>(
    candidates: &[(SourceId, Pal)],
    cat: Category,
    list: impl Fn(&Pal) -> Vec<T>,
    key: impl Fn(&T) -> K,
) -> Vec<T>
where
    K: PartialEq,
{
    let mut ordered: Vec<&(SourceId, Pal)> = candidates.iter().collect();
    ordered.sort_by_key(|(source, _)| cat.rank(*source));
    dedup(ordered.into_iter().flat_map(|(_, p)| list(p)), key)
}

#[must_use]
pub fn pal(candidates: &[(SourceId, Pal)]) -> Option<Pal> {
    let mut ordered: Vec<&(SourceId, Pal)> = candidates.iter().collect();
    ordered.sort_by_key(|(source, _)| Category::Stats.rank(*source));
    let mut base = ordered.first()?.1.clone();

    if let Some(desc) =
        text(candidates, "description", Category::Lore, |p| p.description.clone())
    {
        base.description = Some(desc);
    }

    if base.elements.is_empty() {
        base.elements = union_by_precedence(
            candidates,
            Category::Stats,
            |p| p.elements.clone(),
            |e| *e,
        );
    }
    if base.image_url.is_none() {
        base.image_url =
            text(candidates, "image", Category::Lore, |p| p.image_url.clone());
    }

    base.drops = union_by_precedence(
        candidates,
        Category::Drops,
        |p| p.drops.clone(),
        |d| d.to_lowercase(),
    );
    base.suitability = union_by_precedence(
        candidates,
        Category::Suitability,
        |p| p.suitability.clone(),
        |s| s.kind.to_lowercase(),
    );
    base.active_skills = union_by_precedence(
        candidates,
        Category::Stats,
        |p| p.active_skills.clone(),
        |s| s.name.to_lowercase(),
    );

    Some(base)
}

#[must_use]
pub fn item(candidates: &[(SourceId, Item)]) -> Option<Item> {
    let mut ordered: Vec<&(SourceId, Item)> = candidates.iter().collect();
    ordered.sort_by_key(|(source, _)| Category::Items.rank(*source));
    ordered.first().map(|(_, i)| i.clone())
}

#[must_use]
pub fn passive(candidates: &[(SourceId, PassiveSkill)]) -> Option<PassiveSkill> {
    let mut ordered: Vec<&(SourceId, PassiveSkill)> = candidates.iter().collect();
    ordered.sort_by_key(|(source, _)| Category::Passives.rank(*source));
    ordered.first().map(|(_, p)| p.clone())
}
