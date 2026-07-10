//! Cross-referencing consensus tests: majority vote, precedence tiebreak, and
//! whole-entity merges. No network — pure `merge`/`source` logic.

use marathon::merge::{self, consensus};
use marathon::model::{Stat, Weapon};
use marathon::source::{Category, SourceId};

fn vote(source: SourceId, value: &str) -> (SourceId, Option<String>) {
    (source, Some(value.to_string()))
}

#[test]
fn majority_value_wins_over_precedence() {
    // Lower-precedence sources agree; the single top source disagrees.
    let candidates = [
        vote(SourceId::MarathonDb, "16"), // top precedence for Stats
        vote(SourceId::TauCeti, "18"),
        vote(SourceId::CyberAcme, "18"),
    ];
    let winner = consensus("damage", Category::Stats, &candidates);
    assert_eq!(winner.as_deref(), Some("18"));
}

#[test]
fn tie_broken_by_precedence_rank() {
    // One vote each: MarathonDb outranks Mobalytics in Stats.
    let candidates =
        [vote(SourceId::Mobalytics, "20"), vote(SourceId::MarathonDb, "16")];
    let winner = consensus("damage", Category::Stats, &candidates);
    assert_eq!(winner.as_deref(), Some("16"));
}

#[test]
fn single_source_passes_through() {
    let candidates = [vote(SourceId::TauCeti, "42")];
    assert_eq!(
        consensus("range", Category::Stats, &candidates).as_deref(),
        Some("42")
    );
}

#[test]
fn all_empty_yields_none() {
    let candidates: [(SourceId, Option<String>); 3] = [
        (SourceId::MarathonDb, None),
        (SourceId::Fandom, Some(String::new())),
        (SourceId::TauCeti, Some("   ".to_string())),
    ];
    // Empty/whitespace never reaches consensus if callers pre-clean; here we
    // pass raw Nones and blanks are still distinct non-None strings, so assert
    // the pre-clean helper strips them.
    let cleaned: Vec<_> =
        candidates.into_iter().map(|(s, v)| (s, merge::nonempty(v))).collect();
    assert!(consensus("x", Category::Stats, &cleaned).is_none());
}

#[test]
fn category_rank_orders_sources() {
    assert!(
        Category::Stats.rank(SourceId::MarathonDb)
            < Category::Stats.rank(SourceId::Mobalytics)
    );
    assert!(
        Category::Lore.rank(SourceId::Fandom)
            < Category::Lore.rank(SourceId::MarathonDb)
    );
    assert!(
        Category::Map.rank(SourceId::MapGenie)
            < Category::Map.rank(SourceId::MetaForge)
    );
    // Unlisted source ranks last.
    assert_eq!(
        Category::Meta.rank(SourceId::Fandom),
        Category::Meta.precedence().len()
    );
}

#[test]
fn weapon_merge_cross_references_stats_and_unions_lists() {
    let db = Weapon {
        slug: "assault-rifle".into(),
        name: "Assault Rifle".into(),
        damage: Some("30".into()),
        stats: vec![Stat { name: "Damage".into(), value: "30".into() }],
        ..Default::default()
    };
    let tauceti = Weapon {
        slug: "assault-rifle".into(),
        name: "Assault Rifle".into(),
        damage: Some("32".into()),
        description: Some("A reliable rifle.".into()),
        stats: vec![Stat { name: "Damage".into(), value: "32".into() }, Stat {
            name: "Range".into(),
            value: "40m".into(),
        }],
        ..Default::default()
    };
    let cyberacme = Weapon {
        slug: "assault-rifle".into(),
        name: "Assault Rifle".into(),
        damage: Some("32".into()),
        stats: vec![Stat { name: "Damage".into(), value: "32".into() }],
        ..Default::default()
    };

    let merged = merge::weapon(&[
        (SourceId::MarathonDb, db),
        (SourceId::TauCeti, tauceti),
        (SourceId::CyberAcme, cyberacme),
    ])
    .expect("non-empty candidates");

    assert_eq!(merged.slug, "assault-rifle");
    // 32 has two votes vs MarathonDb's one 30 → majority wins.
    assert_eq!(merged.damage.as_deref(), Some("32"));
    // Description only exists on TauCeti.
    assert_eq!(merged.description.as_deref(), Some("A reliable rifle."));
    // Union of stat names across sources; Damage consensus is 32.
    let damage = merged.stats.iter().find(|s| s.name == "Damage");
    assert_eq!(damage.map(|s| s.value.as_str()), Some("32"));
    assert!(merged.stats.iter().any(|s| s.name == "Range"));
}

#[test]
fn weapon_merge_empty_is_none() {
    assert!(merge::weapon(&[]).is_none());
}
