pub mod catalogue;

use std::collections::{BTreeMap, BTreeSet};

pub use catalogue::{Catalogue, Region, catalogue};

use crate::model::PlayerRoster;
use crate::save::player::PlayerRecord;

const TRANSLATION_X: f64 = 123_930.0;
const TRANSLATION_Y: f64 = 157_935.0;
const SCALE: f64 = 459.0;

#[must_use]
pub fn world_to_map(x: f64, y: f64) -> (i64, i64) {
    (
        round_coord((y - TRANSLATION_Y) / SCALE),
        round_coord((x + TRANSLATION_X) / SCALE),
    )
}

const fn scoped(
    map: Region,
    palpagos: &'static str,
    tree: &'static str,
) -> &'static str {
    match map {
        Region::Palpagos => palpagos,
        Region::WorldTree => tree,
    }
}

fn round_coord(value: f64) -> i64 {
    if !value.is_finite() {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        reason = "clamped to i32 range first, so the i64 cast is exact"
    )]
    {
        value.round().clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i64
    }
}

pub const CAPTURE_TIERS: [i64; 2] = [1, 5];

pub const MAX_STARS: u8 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingEntry {
    pub name: String,
    pub coords: Option<(i64, i64)>,
}

impl MissingEntry {
    fn named(name: impl Into<String>) -> Self {
        Self { name: name.into(), coords: None }
    }

    fn at(name: impl Into<String>, x: f64, y: f64) -> Self {
        Self { name: name.into(), coords: Some(world_to_map(x, y)) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Milestone {
    pub key: &'static str,
    pub category: &'static str,
    pub label: &'static str,
    pub map: Option<Region>,
    pub have: usize,
    pub total: Option<usize>,
    pub note: Option<String>,
    pub weighted: bool,
    pub missing: Vec<MissingEntry>,
}

impl Milestone {
    #[must_use]
    pub fn fraction(&self) -> Option<f64> {
        let total = self.total?;
        if total == 0 {
            return Some(1.0);
        }
        #[expect(
            clippy::cast_precision_loss,
            reason = "counts are catalogue-sized, far below f64's exact range"
        )]
        Some((self.have.min(total) as f64) / (total as f64))
    }

    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.total.is_some_and(|t| self.have >= t)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    pub player: String,
    pub level: i64,
    pub exp: i64,
    pub has_player_save: bool,
    pub game_cleared: bool,
    pub unlocked_maps: Vec<Region>,
    pub milestones: Vec<Milestone>,
}

impl Progress {
    #[must_use]
    pub fn overall(&self) -> f64 {
        let mut by_category: Vec<(&str, usize, usize)> = Vec::new();
        for milestone in self.milestones.iter().filter(|m| m.weighted) {
            let Some(total) = milestone.total else { continue };
            match by_category.iter_mut().find(|(c, ..)| *c == milestone.category) {
                Some((_, have, sum)) => {
                    *have += milestone.have.min(total);
                    *sum += total;
                },
                None => by_category.push((
                    milestone.category,
                    milestone.have.min(total),
                    total,
                )),
            }
        }
        if by_category.is_empty() {
            return 0.0;
        }

        #[expect(
            clippy::cast_precision_loss,
            reason = "catalogue-sized counts, far below f64's exact range"
        )]
        let fractions = by_category.iter().map(|&(_, have, total)| {
            if total == 0 { 1.0 } else { have as f64 / total as f64 }
        });
        #[expect(
            clippy::cast_precision_loss,
            reason = "category count is a small constant"
        )]
        let n = by_category.len() as f64;
        fractions.sum::<f64>() / n
    }

    #[must_use]
    pub fn map_overall(&self, map: Region) -> Option<f64> {
        mean(self.milestones.iter().filter(|m| m.weighted && m.map == Some(map)))
    }

    #[must_use]
    pub fn is_unlocked(&self, map: Region) -> bool {
        self.unlocked_maps.contains(&map)
    }

    pub fn on_map(&self, map: Region) -> impl Iterator<Item = &Milestone> {
        self.milestones.iter().filter(move |m| m.map == Some(map))
    }

    pub fn global(&self) -> impl Iterator<Item = &Milestone> {
        self.milestones.iter().filter(|m| m.map.is_none())
    }

    #[must_use]
    pub fn milestone(&self, key: &str) -> Option<&Milestone> {
        self.milestones.iter().find(|m| m.key == key)
    }
}

fn mean<'a>(milestones: impl Iterator<Item = &'a Milestone>) -> Option<f64> {
    let fractions: Vec<f64> = milestones.filter_map(Milestone::fraction).collect();
    if fractions.is_empty() {
        return None;
    }
    #[expect(
        clippy::cast_precision_loss,
        reason = "milestone count is a small constant"
    )]
    let n = fractions.len() as f64;
    Some(fractions.iter().sum::<f64>() / n)
}

#[must_use]
pub fn compute(
    record: Option<&PlayerRecord>,
    roster: &PlayerRoster,
    cat: &Catalogue,
) -> Progress {
    let mut milestones = Vec::new();

    if let Some(record) = record {
        for map in Region::ALL {
            milestones.extend([
                fast_travel(record, cat, map),
                towers(record, cat, map),
                bosses(record, cat, map),
                bounties(record, cat, map),
                effigies(record, cat, map),
                relics(record, cat, map),
            ]);
        }
        milestones.extend([
            areas(record, cat),
            paldeck(record, cat),
            captures(record, cat),
            technology(record, cat),
            missions(record, cat),
        ]);
        milestones.push(capture_bonus(record, cat));
        milestones.extend([
            arena(record),
            notes(record),
            exploration(record),
            world_deeds(record),
        ]);
    }

    milestones.extend([condensed(roster, cat), collection(roster, cat)]);

    Progress {
        player: roster.name.clone(),
        level: roster.level,
        exp: roster.exp,
        has_player_save: record.is_some(),
        game_cleared: record.is_some_and(|r| r.game_cleared),
        unlocked_maps: record.map_or_else(Vec::new, unlocked_maps),
        milestones,
    }
}

fn unlocked_maps(record: &PlayerRecord) -> Vec<Region> {
    Region::ALL
        .into_iter()
        .filter(|map| record.world_maps.contains(map.unlock_flag()))
        .collect()
}

fn fast_travel(record: &PlayerRecord, cat: &Catalogue, map: Region) -> Milestone {
    let points: Vec<_> = cat.fast_travel_on(map).collect();
    let missing: Vec<MissingEntry> = points
        .iter()
        .filter(|p| !record.fast_travel.contains(&p.id))
        .map(|p| MissingEntry::at(&p.name, p.x, p.y))
        .collect();

    let statues = points.iter().filter(|p| p.kind == "tower").count();
    Milestone {
        key: scoped(map, "fast-travel", "tree-fast-travel"),
        category: "fast-travel",
        label: "Fast travel points",
        map: Some(map),
        have: points.len() - missing.len(),
        total: Some(points.len()),
        note: Some(format!(
            "{statues} statues, {} map points",
            points.len() - statues
        )),
        weighted: true,
        missing,
    }
}

fn towers(record: &PlayerRecord, cat: &Catalogue, map: Region) -> Milestone {
    let towers: Vec<_> = cat.towers_on(map).collect();
    let missing: Vec<MissingEntry> = towers
        .iter()
        .filter(|t| !record.towers_defeated.contains(&t.flag))
        .map(|t| MissingEntry::named(format!("{} — {}", t.name, t.location)))
        .collect();

    Milestone {
        key: scoped(map, "towers", "tree-towers"),
        category: "towers",
        label: "Tower bosses",
        map: Some(map),
        have: towers.len() - missing.len(),
        total: Some(towers.len()),
        note: None,
        weighted: true,
        missing,
    }
}

fn defeated_spawners(record: &PlayerRecord) -> BTreeSet<String> {
    record.bosses_defeated.iter().map(|s| s.to_lowercase()).collect()
}

fn bosses(record: &PlayerRecord, cat: &Catalogue, map: Region) -> Milestone {
    let defeated = defeated_spawners(record);
    let bosses: Vec<_> = cat.bosses_on(map).collect();
    let missing: Vec<MissingEntry> = bosses
        .iter()
        .filter(|b| !defeated.contains(&b.spawner.to_lowercase()))
        .map(|b| MissingEntry::at(format!("{} (Lv {})", b.name, b.level), b.x, b.y))
        .collect();

    let alphas = bosses.iter().filter(|b| b.alpha).count();
    Milestone {
        key: scoped(map, "bosses", "tree-bosses"),
        category: "bosses",
        label: "Field & alpha bosses",
        map: Some(map),
        have: bosses.len() - missing.len(),
        total: Some(bosses.len()),
        note: Some(match map {
            Region::Palpagos => format!("{alphas} alphas, includes sealed realms"),
            Region::WorldTree => format!("{alphas} alphas"),
        }),
        weighted: true,
        missing,
    }
}

fn bounties(record: &PlayerRecord, cat: &Catalogue, map: Region) -> Milestone {
    let defeated = defeated_spawners(record);
    let targets: Vec<_> = cat.bosses_on(map).filter(|b| b.bounty).collect();
    let missing: Vec<MissingEntry> = targets
        .iter()
        .filter(|b| !defeated.contains(&b.spawner.to_lowercase()))
        .map(|b| MissingEntry::at(&b.name, b.x, b.y))
        .collect();

    Milestone {
        key: scoped(map, "bounties", "tree-bounties"),
        category: "bounties",
        label: "Bounty targets",
        map: Some(map),
        have: targets.len() - missing.len(),
        total: Some(targets.len()),
        note: Some("bosses that drop a bounty token".to_string()),
        weighted: true,
        missing,
    }
}

fn effigies(record: &PlayerRecord, cat: &Catalogue, map: Region) -> Milestone {
    let effigies: Vec<_> = cat.effigies_on(map).collect();
    let missing: Vec<MissingEntry> = effigies
        .iter()
        .filter(|r| !record.effigies.contains(&r.id))
        .map(|r| match (r.x, r.y) {
            (Some(x), Some(y)) => MissingEntry::at("Effigy", x, y),
            _ => MissingEntry::named("Effigy (location unknown)"),
        })
        .collect();

    Milestone {
        key: scoped(map, "effigies", "tree-effigies"),
        category: "effigies",
        label: "Lifmunk effigies",
        map: Some(map),
        have: effigies.len() - missing.len(),
        total: Some(effigies.len()),
        note: None,
        weighted: true,
        missing,
    }
}

fn relics(record: &PlayerRecord, cat: &Catalogue, map: Region) -> Milestone {
    let mut have = 0usize;
    let mut total = 0usize;
    let mut missing = Vec::new();

    for relic_type in &cat.relic_types {
        let available: BTreeSet<&str> = cat
            .relics_on(map)
            .filter(|r| r.kind == relic_type.key)
            .map(|r| r.id.as_str())
            .collect();
        if available.is_empty() {
            continue;
        }
        let collected =
            record.relics_by_type.get(&relic_type.key).map_or(0, |got| {
                available.iter().filter(|id| got.contains(**id)).count()
            });

        have += collected;
        total += available.len();
        if collected < available.len() {
            missing.push(MissingEntry::named(format!(
                "{}: {collected}/{}",
                relic_type.name,
                available.len()
            )));
        }
    }

    let unspent: i64 = record.relics_unspent.values().sum();
    let unspent_note = if map == Region::Palpagos && unspent > 0 {
        format!("{unspent} unspent; ")
    } else {
        String::new()
    };

    Milestone {
        key: scoped(map, "relics", "tree-relics"),
        category: "relics",
        label: "Stat relics",
        map: Some(map),
        have,
        total: Some(total),
        note: Some(format!("{unspent_note}effigies are the Capture Power relics")),
        weighted: true,
        missing,
    }
}

fn paldeck(record: &PlayerRecord, cat: &Catalogue) -> Milestone {
    let seen: BTreeSet<&str> = record
        .paldeck_seen
        .iter()
        .filter_map(|id| cat.pal(id).map(|p| p.id.as_str()))
        .collect();

    let missing: Vec<MissingEntry> = cat
        .pals
        .iter()
        .filter(|p| !seen.contains(p.id.as_str()))
        .map(|p| MissingEntry::named(format!("#{:03} {}", p.dex, p.name)))
        .collect();

    Milestone {
        key: "paldeck",
        category: "paldeck",
        label: "Paldeck entries seen",
        map: None,
        have: seen.len(),
        total: Some(cat.pals.len()),
        note: Some("includes variants".to_string()),
        weighted: true,
        missing,
    }
}

fn captures(record: &PlayerRecord, cat: &Catalogue) -> Milestone {
    let caught = caught_species(record, cat);
    let missing: Vec<MissingEntry> = cat
        .pals
        .iter()
        .filter(|p| !caught.contains(p.id.as_str()))
        .map(|p| MissingEntry::named(format!("#{:03} {}", p.dex, p.name)))
        .collect();

    let total_catches: i64 = record.pal_captures.values().sum();
    Milestone {
        key: "captures",
        category: "captures",
        label: "Species caught at least once",
        map: None,
        have: caught.len(),
        total: Some(cat.pals.len()),
        note: Some(format!("{total_catches} captures in total")),
        weighted: true,
        missing,
    }
}

fn caught_species<'a>(
    record: &PlayerRecord,
    cat: &'a Catalogue,
) -> BTreeSet<&'a str> {
    record
        .pal_captures
        .iter()
        .filter(|(_, count)| **count > 0)
        .filter_map(|(id, _)| cat.pal(id).map(|p| p.id.as_str()))
        .collect()
}

fn capture_bonus(record: &PlayerRecord, cat: &Catalogue) -> Milestone {
    let mut by_species: BTreeMap<&str, i64> = BTreeMap::new();
    for (id, count) in &record.pal_captures {
        if let Some(pal) = cat.pal(id) {
            *by_species.entry(pal.id.as_str()).or_default() += *count;
        }
    }

    let threshold = CAPTURE_TIERS[1];
    let have = by_species.values().filter(|n| **n >= threshold).count();
    let missing: Vec<MissingEntry> = cat
        .pals
        .iter()
        .map(|p| (p, by_species.get(p.id.as_str()).copied().unwrap_or(0)))
        .filter(|(_, at)| *at < threshold)
        .map(|(p, at)| MissingEntry::named(format!("{} ({at}/{threshold})", p.name)))
        .collect();

    Milestone {
        key: "captures-5x",
        category: "captures-5x",
        label: "Species caught 5x",
        map: None,
        have,
        total: Some(cat.pals.len()),
        note: Some("unlocks the full capture bonus".to_string()),
        weighted: false,
        missing,
    }
}

fn condensed(roster: &PlayerRoster, cat: &Catalogue) -> Milestone {
    let mut best: BTreeMap<&str, u8> = BTreeMap::new();
    for pal in &roster.personal_pals {
        if let Some(entry) = cat.pal(&pal.species) {
            let slot = best.entry(entry.id.as_str()).or_default();
            *slot = (*slot).max(pal.stars);
        }
    }

    let have = best.values().filter(|s| **s >= MAX_STARS).count();
    let missing: Vec<MissingEntry> = cat
        .pals
        .iter()
        .map(|p| (p, best.get(p.id.as_str()).copied()))
        .filter(|(_, stars)| stars.is_none_or(|s| s < MAX_STARS))
        .map(|(p, stars)| {
            MissingEntry::named(stars.map_or_else(
                || format!("{} (not owned)", p.name),
                |s| format!("{} ({s}★/{MAX_STARS}★)", p.name),
            ))
        })
        .collect();

    let fully = roster.personal_pals.iter().filter(|p| p.stars >= MAX_STARS).count();
    let partly = roster
        .personal_pals
        .iter()
        .filter(|p| p.stars > 0 && p.stars < MAX_STARS)
        .count();

    let mut note = vec![format!(
        "{fully} Pal{} at {MAX_STARS}★",
        if fully == 1 { "" } else { "s" }
    )];
    if partly > 0 {
        note.push(format!("{partly} partly condensed"));
    }

    Milestone {
        key: "condensed",
        category: "condensed",
        label: "Species fully condensed",
        map: None,
        have,
        total: Some(cat.pals.len()),
        note: Some(note.join(", ")),
        weighted: false,
        missing,
    }
}

fn technology(record: &PlayerRecord, cat: &Catalogue) -> Milestone {
    let missing: Vec<MissingEntry> = cat
        .technologies
        .iter()
        .filter(|t| !record.technologies.contains(&t.id))
        .map(|t| {
            let kind = if t.boss { "boss" } else { "Lv " };
            MissingEntry::named(if t.boss {
                format!("{} ({kind})", t.name)
            } else {
                format!("{} ({kind}{})", t.name, t.level)
            })
        })
        .collect();

    let unspent = record.technology_points + record.boss_technology_points;
    Milestone {
        key: "technology",
        category: "technology",
        label: "Technology unlocked",
        map: None,
        have: cat.technologies.len() - missing.len(),
        total: Some(cat.technologies.len()),
        note: (unspent > 0).then(|| {
            format!(
                "{} points and {} boss points unspent",
                record.technology_points, record.boss_technology_points
            )
        }),
        weighted: true,
        missing,
    }
}

fn missions(record: &PlayerRecord, cat: &Catalogue) -> Milestone {
    let tracked: Vec<_> = cat.tracked_missions().collect();
    let missing: Vec<MissingEntry> = tracked
        .iter()
        .filter(|m| !record.quests_completed.contains(&m.id))
        .map(|m| MissingEntry::named(format!("{} ({})", m.name, m.kind)))
        .collect();

    let active = record.quests_active.len();
    Milestone {
        key: "missions",
        category: "missions",
        label: "Missions completed",
        map: None,
        have: tracked.len() - missing.len(),
        total: Some(tracked.len()),
        note: (active > 0).then(|| format!("{active} in progress")),
        weighted: true,
        missing,
    }
}

fn areas(record: &PlayerRecord, cat: &Catalogue) -> Milestone {
    let found: BTreeSet<String> =
        record.areas_found.iter().map(|a| a.to_lowercase()).collect();
    let missing: Vec<MissingEntry> = cat
        .areas
        .iter()
        .filter(|a| !found.contains(&a.to_lowercase()))
        .map(|a| MissingEntry::named(a.replace('_', " ")))
        .collect();

    Milestone {
        key: "areas",
        category: "areas",
        label: "Map areas discovered",
        map: Some(Region::Palpagos),
        have: cat.areas.len() - missing.len(),
        total: Some(cat.areas.len()),
        note: None,
        weighted: true,
        missing,
    }
}

const ARENA_RANKS: [&str; 7] =
    ["Bronze", "Silver", "Gold", "Platinum", "Diamond", "Master", "Legend"];

fn arena(record: &PlayerRecord) -> Milestone {
    let cleared: Vec<&str> = record
        .arena_ranks
        .iter()
        .filter(|(_, clears)| **clears > 0)
        .map(|(rank, _)| rank.as_str())
        .collect();

    let missing: Vec<MissingEntry> = ARENA_RANKS
        .iter()
        .filter(|rank| !cleared.contains(*rank))
        .map(|rank| MissingEntry::named(*rank))
        .collect();

    let unknown = cleared.iter().filter(|r| !ARENA_RANKS.contains(*r)).count();

    Milestone {
        key: "arena",
        category: "arena",
        label: "Arena ranks cleared",
        map: None,
        have: cleared.len(),
        total: Some(ARENA_RANKS.len() + unknown),
        note: (!cleared.is_empty()).then(|| cleared.join(", ")),
        weighted: false,
        missing,
    }
}

fn notes(record: &PlayerRecord) -> Milestone {
    Milestone {
        key: "notes",
        category: "notes",
        label: "Lore notes found",
        map: None,
        have: record.notes.len(),
        total: None,
        note: Some("no known total".to_string()),
        weighted: false,
        missing: Vec::new(),
    }
}

fn exploration(record: &PlayerRecord) -> Milestone {
    let dungeons = record.normal_dungeons_cleared + record.fixed_dungeons_cleared;
    Milestone {
        key: "dungeons",
        category: "dungeons",
        label: "Dungeons cleared",
        map: None,
        have: usize::try_from(dungeons).unwrap_or(0),
        total: None,
        note: Some(format!(
            "{} normal, {} sealed realm",
            record.normal_dungeons_cleared, record.fixed_dungeons_cleared
        )),
        weighted: false,
        missing: Vec::new(),
    }
}

fn world_deeds(record: &PlayerRecord) -> Milestone {
    let deeds = [
        ("raider camp", "raider camps", record.camps_conquered),
        ("oil rig", "oil rigs", record.oilrigs_cleared),
        ("treasure", "treasures", record.treasures()),
        ("predator", "predators", record.predators_defeated),
        ("awakening", "awakenings", record.awakenings),
        ("mutation", "mutations", record.mutations),
    ];
    let total: i64 = deeds.iter().map(|(_, _, n)| n).sum();
    let note = deeds
        .iter()
        .filter(|(_, _, n)| *n > 0)
        .map(|(one, many, n)| format!("{n} {}", if *n == 1 { one } else { many }))
        .collect::<Vec<_>>()
        .join(", ");

    Milestone {
        key: "deeds",
        category: "deeds",
        label: "World deeds",
        map: None,
        have: usize::try_from(total).unwrap_or(0),
        total: None,
        note: (!note.is_empty()).then_some(note),
        weighted: false,
        missing: Vec::new(),
    }
}

fn collection(roster: &PlayerRoster, cat: &Catalogue) -> Milestone {
    let owned = &roster.personal_pals;
    let alphas = owned.iter().filter(|p| p.is_alpha).count();
    let luckies = owned.iter().filter(|p| p.is_lucky).count();
    let species: BTreeSet<&str> = owned
        .iter()
        .filter_map(|p| cat.pal(&p.species).map(|e| e.id.as_str()))
        .collect();

    let mut note = vec![format!("{} species", species.len())];
    if alphas > 0 {
        note.push(format!("{alphas} alpha"));
    }
    if luckies > 0 {
        note.push(format!("{luckies} lucky"));
    }

    Milestone {
        key: "collection",
        category: "collection",
        label: "Pals owned",
        map: None,
        have: owned.len(),
        total: None,
        note: Some(note.join(", ")),
        weighted: false,
        missing: Vec::new(),
    }
}
