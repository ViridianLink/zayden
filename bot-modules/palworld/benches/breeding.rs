use std::collections::HashMap;
use std::fs;
use std::hint::black_box;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use palworld::breeding::BreedingIndex;
use palworld::difficulty::pal_difficulty;
use palworld::model::{Gender, OwnedPal, Pal};
use palworld::parse::pal_from_palcalc;
use palworld::transport::{BreedingMap, parse_breeding, parse_pals};

/// A pal with no wild spawn, so the planner has to breed its way there rather
/// than short-circuiting on a catch.
const TARGET: &str = "NightLady";

fn fixture(name: &str) -> String {
    let path = format!("{}/tests/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&path).unwrap_or_default()
}

fn pals() -> Vec<Pal> {
    parse_pals(&fixture("palcalc_db.json"))
        .unwrap_or_default()
        .into_iter()
        .map(pal_from_palcalc)
        .collect()
}

fn breeding_map() -> BreedingMap {
    parse_breeding(&fixture("palcalc_breeding.json")).unwrap_or_default()
}

/// Six species, both genders each - a plausible early-game roster, and enough
/// owned leaves that the search has real alternatives to weigh.
fn owned_roster(pals: &[Pal]) -> Vec<OwnedPal> {
    pals.iter()
        .filter(|p| p.key != TARGET)
        .take(6)
        .flat_map(|p| {
            [Gender::Male, Gender::Female].map(|gender| OwnedPal {
                species: p.key.clone(),
                gender,
                ..OwnedPal::default()
            })
        })
        .collect()
}

fn breeding(c: &mut Criterion) {
    let map = breeding_map();
    assert!(!map.is_empty(), "palcalc_breeding.json fixture is missing");

    let pals = pals();
    assert!(!pals.is_empty(), "palcalc_db.json fixture is missing");

    let base_cost: HashMap<String, i64> =
        pals.iter().map(|p| (p.key.clone(), pal_difficulty(p))).collect();
    let owned = owned_roster(&pals);
    let index = BreedingIndex::from_map(map.clone());

    assert!(
        index.plan(&owned, TARGET, &base_cost).is_some(),
        "{TARGET} must stay reachable or the plan bench measures an early return",
    );

    let mut group = c.benchmark_group("breeding");
    group.bench_function("from_map", |b| {
        b.iter_batched(
            || map.clone(),
            BreedingIndex::from_map,
            BatchSize::SmallInput,
        );
    });
    group.bench_function("plan", |b| {
        b.iter(|| {
            index.plan(black_box(&owned), black_box(TARGET), black_box(&base_cost))
        });
    });
    group.bench_function("breed_for", |b| {
        b.iter(|| index.breed_for(black_box(TARGET)));
    });
    group.finish();
}

criterion_group!(benches, breeding);
criterion_main!(benches);
