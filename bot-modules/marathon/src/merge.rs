use std::collections::HashMap;
use std::hash::Hash;

use crate::model::{Faction, MarathonMap, Runner, Stat, Weapon};
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

fn union<T, K, F>(primary: &mut Vec<T>, other: Vec<T>, key: F)
where
    K: PartialEq,
    F: Fn(&T) -> K,
{
    let existing = std::mem::take(primary);
    *primary = dedup(existing.into_iter().chain(other), key);
}

fn fill(primary: &mut Option<String>, other: Option<String>) {
    if primary.as_deref().is_none_or(|s| s.trim().is_empty()) {
        *primary = other;
    }
}

pub trait Merge {
    fn merge_from(&mut self, other: Self);
}

impl Merge for MarathonMap {
    fn merge_from(&mut self, other: Self) {
        if self.name.trim().is_empty() {
            self.name = other.name;
        }
        if self.status.is_none() {
            self.status = other.status;
        }
        fill(&mut self.map_image_url, other.map_image_url);

        union(&mut self.pois, other.pois, |p| p.name.to_lowercase());
        union(&mut self.extractions, other.extractions, |l| l.name.to_lowercase());
        union(&mut self.event_spawns, other.event_spawns, |l| l.name.to_lowercase());
        union(&mut self.keycard_rooms, other.keycard_rooms, |r| {
            r.name.to_lowercase()
        });
    }
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

    let winner = tally
        .iter()
        .max_by(|a, b| a.1.0.cmp(&b.1.0).then_with(|| b.1.1.cmp(&a.1.1)))
        .map(|(value, _)| (*value).clone())?;

    if tally.len() > 1
        && let Some((_, top)) = candidates
            .iter()
            .filter(|(_, v)| v.is_some())
            .min_by_key(|(source, _)| category.rank(*source))
        && top.as_ref() != Some(&winner)
    {
        tracing::warn!(field, ?category, "consensus overrode top-precedence source");
    }

    Some(winner)
}

fn collect<E, T, F>(candidates: &[(SourceId, E)], f: F) -> Vec<(SourceId, Option<T>)>
where
    F: Fn(&E) -> Option<T>,
{
    candidates.iter().map(|(source, entity)| (*source, f(entity))).collect()
}

fn text<E, F>(
    candidates: &[(SourceId, E)],
    field: &str,
    cat: Category,
    f: F,
) -> Option<String>
where
    F: Fn(&E) -> Option<String>,
{
    consensus(field, cat, &collect(candidates, |e| nonempty(f(e))))
}

#[must_use]
pub fn weapon(candidates: &[(SourceId, Weapon)]) -> Option<Weapon> {
    candidates.first()?;
    let slug = slug_of(candidates, |w| &w.slug);

    let name = text(candidates, "name", Category::Stats, |w| Some(w.name.clone()))
        .unwrap_or_else(|| slug.clone());

    Some(Weapon {
        weapon_type: text(candidates, "weapon_type", Category::Stats, |w| {
            w.weapon_type.clone()
        }),
        ammo_type: text(candidates, "ammo_type", Category::Stats, |w| {
            w.ammo_type.clone()
        }),
        damage: text(candidates, "damage", Category::Stats, |w| w.damage.clone()),
        fire_rate: text(candidates, "fire_rate", Category::Stats, |w| {
            w.fire_rate.clone()
        }),
        magazine_size: text(candidates, "magazine_size", Category::Stats, |w| {
            w.magazine_size.clone()
        }),
        reload_speed: text(candidates, "reload_speed", Category::Stats, |w| {
            w.reload_speed.clone()
        }),
        range: text(candidates, "range", Category::Stats, |w| w.range.clone()),
        description: text(candidates, "description", Category::Lore, |w| {
            w.description.clone()
        }),
        thumbnail_url: text(candidates, "thumbnail_url", Category::Stats, |w| {
            w.thumbnail_url.clone()
        }),
        stats: merge_stats(candidates, |w| &w.stats, Category::Stats),
        attachment_slots: union_by_precedence(
            candidates,
            Category::Attachments,
            |w| w.attachment_slots.clone(),
            |s| s.slot.to_lowercase(),
        ),
        slug,
        name,
    })
}

#[must_use]
pub fn runner(candidates: &[(SourceId, Runner)]) -> Option<Runner> {
    candidates.first()?;
    let slug = slug_of(candidates, |r| &r.slug);

    let name = text(candidates, "name", Category::Stats, |r| Some(r.name.clone()))
        .unwrap_or_else(|| slug.clone());

    Some(Runner {
        role: text(candidates, "role", Category::Stats, |r| r.role.clone()),
        description: text(candidates, "description", Category::Lore, |r| {
            r.description.clone()
        }),
        portrait_url: text(candidates, "portrait_url", Category::Stats, |r| {
            r.portrait_url.clone()
        }),
        abilities: union_by_precedence(
            candidates,
            Category::Stats,
            |r| r.abilities.clone(),
            |a| a.name.to_lowercase(),
        ),
        cores: union_by_precedence(
            candidates,
            Category::Stats,
            |r| r.cores.clone(),
            |c| c.to_lowercase(),
        ),
        stats: merge_stats(candidates, |r| &r.stats, Category::Stats),
        slug,
        name,
    })
}

#[must_use]
pub fn faction(candidates: &[(SourceId, Faction)]) -> Option<Faction> {
    candidates.first()?;
    let slug = slug_of(candidates, |f| &f.slug);

    let name = text(candidates, "name", Category::Faction, |f| Some(f.name.clone()))
        .unwrap_or_else(|| slug.clone());

    Some(Faction {
        priority_contracts: union_by_precedence(
            candidates,
            Category::Faction,
            |f| f.priority_contracts.clone(),
            |c| c.slug.to_lowercase(),
        ),
        upgrades: union_by_precedence(
            candidates,
            Category::Faction,
            |f| f.upgrades.clone(),
            |u| u.name.to_lowercase(),
        ),
        slug,
        name,
    })
}

fn slug_of<E>(candidates: &[(SourceId, E)], f: impl Fn(&E) -> &String) -> String {
    candidates
        .iter()
        .map(|(_, e)| f(e))
        .find(|s| !s.trim().is_empty())
        .cloned()
        .unwrap_or_default()
}

fn union_by_precedence<E, T, K>(
    candidates: &[(SourceId, E)],
    cat: Category,
    list: impl Fn(&E) -> Vec<T>,
    key: impl Fn(&T) -> K,
) -> Vec<T>
where
    K: PartialEq,
{
    let mut ordered: Vec<&(SourceId, E)> = candidates.iter().collect();
    ordered.sort_by_key(|(source, _)| cat.rank(*source));
    dedup(ordered.into_iter().flat_map(|(_, e)| list(e)), key)
}

fn merge_stats<E>(
    candidates: &[(SourceId, E)],
    stats: impl Fn(&E) -> &Vec<Stat>,
    cat: Category,
) -> Vec<Stat> {
    let mut ordered: Vec<&(SourceId, E)> = candidates.iter().collect();
    ordered.sort_by_key(|(source, _)| cat.rank(*source));

    let names: Vec<String> = dedup(
        ordered.iter().flat_map(|(_, e)| stats(e).iter().map(|s| s.name.clone())),
        |n| n.to_lowercase(),
    );

    names
        .into_iter()
        .filter_map(|name| {
            let votes = collect(candidates, |e| {
                nonempty(
                    stats(e)
                        .iter()
                        .find(|s| s.name.eq_ignore_ascii_case(&name))
                        .map(|s| s.value.clone()),
                )
            });
            consensus(&name, cat, &votes).map(|value| Stat { name, value })
        })
        .collect()
}
