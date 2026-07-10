//! Consensus merge behaviour across sources.

use palworld::merge;
use palworld::model::{Pal, Stats};
use palworld::source::SourceId;

fn paldex_base() -> Pal {
    Pal {
        key: "001".to_string(),
        name: "Lamball".to_string(),
        stats: Some(Stats { hp: 70, ..Stats::default() }),
        drops: vec!["wool".to_string()],
        description: None,
        ..Pal::default()
    }
}

#[test]
fn lore_source_backfills_missing_description() {
    let fandom = Pal {
        key: "001".to_string(),
        name: "Lamball".to_string(),
        description: Some("A fluffy Pal.".to_string()),
        ..Pal::default()
    };

    let merged =
        merge::pal(&[(SourceId::Paldex, paldex_base()), (SourceId::Fandom, fandom)])
            .expect("non-empty candidates");

    // Description comes from the lore source; typed data stays from Paldex.
    assert_eq!(merged.description.as_deref(), Some("A fluffy Pal."));
    assert!(merged.stats.is_some_and(|s| s.hp == 70));
    assert_eq!(merged.drops, vec!["wool".to_string()]);
}

#[test]
fn paldex_only_is_returned_unchanged() {
    let merged =
        merge::pal(&[(SourceId::Paldex, paldex_base())]).expect("candidate");
    assert_eq!(merged.name, "Lamball");
    assert!(merged.description.is_none());
}

#[test]
fn empty_candidates_yield_none() {
    assert!(merge::pal(&[]).is_none());
}
