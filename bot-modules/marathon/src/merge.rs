use crate::model::MarathonMap;

pub trait Merge {
    fn merge_from(&mut self, other: Self);
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
