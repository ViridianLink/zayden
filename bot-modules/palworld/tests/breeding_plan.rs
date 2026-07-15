//! `BreedingIndex::plan` - the pure AND/OR breeding-path search (Milestone 4).
//! No save file needed: the index, owned roster, and base costs are all
//! hand-built, so these assert the algorithm in isolation.

use std::collections::HashMap;

use palworld::breeding::BreedingIndex;
use palworld::model::{Gender, OwnedPal};

fn pal(species: &str, gender: Gender) -> OwnedPal {
    OwnedPal { species: species.to_string(), gender, nickname: None }
}

fn base_costs(entries: &[(&str, i64)]) -> HashMap<String, i64> {
    entries.iter().map(|(k, v)| ((*k).to_string(), *v)).collect()
}

/// Build an index from `child → [parent pairs]`.
fn index(entries: &[(&str, &[(&str, &str)])]) -> BreedingIndex {
    let mut map: HashMap<String, Vec<[String; 2]>> = HashMap::new();
    for (child, pairs) in entries {
        let combos = pairs
            .iter()
            .map(|(a, b)| [(*a).to_string(), (*b).to_string()])
            .collect();
        map.insert((*child).to_string(), combos);
    }
    BreedingIndex::from_map(map)
}

#[test]
fn breeds_target_from_owned_parents_for_free() {
    let index = index(&[("C", &[("A", "B")])]);
    let owned = vec![pal("A", Gender::Male), pal("B", Gender::Female)];
    let base = base_costs(&[("A", 5), ("B", 5), ("C", 50)]);

    let plan = index.plan(&owned, "C", &base).expect("reachable");

    // Owned parents are free; only the single breeding step is charged.
    assert_eq!(plan.total_cost, 1);
    assert_eq!(plan.steps.len(), 1);
    let step = plan.steps.first().expect("one step");
    assert_eq!(step.child, "C");
    assert!(step.ready, "opposite-gender owned pair is ready now");
    assert!(plan.leaves_to_obtain.is_empty());

    let p = &step.pair;
    assert!((p.a == "A" && p.b == "B") || (p.a == "B" && p.b == "A"));
}

#[test]
fn still_breeds_target_when_catching_is_cheaper() {
    let index = index(&[("C", &[("A", "B")])]);
    let base = base_costs(&[("A", 5), ("B", 5), ("C", 3)]);

    let plan = index.plan(&[], "C", &base).expect("catchable");

    // Catching C (3) beats breeding it (5 + 5 + 1 = 11), but a breed-plan must
    // still show the breeding path: one step producing C, with the cheaper
    // catch cost recorded so the embed can note it.
    assert_eq!(plan.total_cost, 11);
    assert_eq!(plan.catch_cost, Some(3));
    let step = plan.steps.last().expect("a breeding step");
    assert_eq!(step.child, "C");
    let p = &step.pair;
    assert!((p.a == "A" && p.b == "B") || (p.a == "B" && p.b == "A"));
    // Both parents are unowned and must be obtained.
    let mut leaves = plan.leaves_to_obtain.clone();
    leaves.sort();
    assert_eq!(leaves, vec!["A".to_string(), "B".to_string()]);
}

#[test]
fn catch_only_target_has_no_breeding_step() {
    // C has no recipe at all - genuinely catch-only, so no step can be shown.
    let index = index(&[("D", &[("A", "B")])]);
    let base = base_costs(&[("A", 5), ("B", 5), ("C", 3), ("D", 9)]);

    let plan = index.plan(&[], "C", &base).expect("catchable");

    assert_eq!(plan.total_cost, 3);
    assert!(plan.steps.is_empty());
    assert_eq!(plan.catch_cost, None);
    assert_eq!(plan.leaves_to_obtain, vec!["C".to_string()]);
}

#[test]
fn builds_multi_hop_tree_in_dependency_order() {
    let index = index(&[("C", &[("A", "B")]), ("D", &[("C", "X")])]);
    let owned = vec![
        pal("A", Gender::Male),
        pal("B", Gender::Female),
        pal("X", Gender::Male),
    ];
    let base = base_costs(&[("A", 5), ("B", 5), ("X", 5), ("C", 99), ("D", 99)]);

    let plan = index.plan(&owned, "D", &base).expect("reachable");

    // C = 0+0+1, D = cost(C)+0+1 = 2.
    assert_eq!(plan.total_cost, 2);
    let children: Vec<&str> = plan.steps.iter().map(|s| s.child.as_str()).collect();
    assert_eq!(children, vec!["C", "D"], "leaves→target order");

    let c_step = plan.steps.first().expect("C step");
    let d_step = plan.steps.get(1).expect("D step");
    assert!(c_step.ready, "C breeds from owned opposite-gender parents");
    assert!(!d_step.ready, "D needs the bred C in hand first");
    assert!(plan.leaves_to_obtain.is_empty());
}

#[test]
fn ranks_cheapest_parent_pair_first() {
    // Two routes to C; the expensive (E,F) pair is listed first to prove the
    // search - not input order - picks the cheaper owned (A,B) pair.
    let index = index(&[("C", &[("E", "F"), ("A", "B")])]);
    let owned = vec![pal("A", Gender::Female), pal("B", Gender::Male)];
    let base = base_costs(&[("A", 5), ("B", 5), ("E", 5), ("F", 5), ("C", 99)]);

    let plan = index.plan(&owned, "C", &base).expect("reachable");

    assert_eq!(plan.total_cost, 1);
    let p = &plan.steps.first().expect("one step").pair;
    let got = [p.a.as_str(), p.b.as_str()];
    assert!(got == ["A", "B"] || got == ["B", "A"], "chose {got:?}");
}

#[test]
fn same_species_pair_ready_needs_both_genders() {
    let index = index(&[("C", &[("A", "A")])]);
    let base = base_costs(&[("A", 5), ("C", 99)]);

    let mixed = vec![pal("A", Gender::Male), pal("A", Gender::Female)];
    let plan = index.plan(&mixed, "C", &base).expect("reachable");
    assert_eq!(plan.total_cost, 1);
    assert!(
        plan.steps.first().expect("one step").ready,
        "one male + one female A is ready",
    );

    // Two males still yield the same (free) tree - extras can be gender-flipped
    // by breeding - but the step is not ready to run as-is.
    let same_gender = vec![pal("A", Gender::Male), pal("A", Gender::Male)];
    let plan = index.plan(&same_gender, "C", &base).expect("reachable");
    assert_eq!(plan.total_cost, 1);
    assert!(!plan.steps.first().expect("one step").ready);
    assert!(plan.leaves_to_obtain.is_empty());
}

#[test]
fn returns_none_when_target_unreachable() {
    let index = index(&[("C", &[("A", "B")])]);

    // A is neither owned nor catchable, so the only pair can never fire and C
    // has no base cost of its own.
    let base = base_costs(&[("B", 5)]);
    assert!(index.plan(&[], "C", &base).is_none());

    // A target absent from the roster, base costs, and breeding graph.
    assert!(index.plan(&[], "Z", &base).is_none());
}

#[test]
fn owned_target_both_genders_breeds_from_itself() {
    // Self-pair recipe present and both genders owned → just pair them up.
    let index = index(&[("C", &[("C", "C"), ("A", "B")])]);
    let owned = vec![pal("C", Gender::Male), pal("C", Gender::Female)];
    let base = base_costs(&[("A", 5), ("B", 5), ("C", 50)]);

    let plan = index.plan(&owned, "C", &base).expect("owned");

    assert_eq!(plan.total_cost, 1);
    let step = plan.steps.first().expect("one step");
    assert_eq!(step.child, "C");
    assert_eq!([step.pair.a.as_str(), step.pair.b.as_str()], ["C", "C"]);
    assert!(step.ready, "own both genders - the self-pair is ready");
    assert!(plan.leaves_to_obtain.is_empty());
}

#[test]
fn owned_target_single_gender_uses_another_combination() {
    // Only a male C owned, so C × C can't fire - fall back to the A × B route.
    let index = index(&[("C", &[("C", "C"), ("A", "B")])]);
    let owned = vec![
        pal("C", Gender::Male),
        pal("A", Gender::Male),
        pal("B", Gender::Female),
    ];
    let base = base_costs(&[("A", 5), ("B", 5), ("C", 50)]);

    let plan = index.plan(&owned, "C", &base).expect("owned");

    assert_eq!(plan.total_cost, 1, "owned A + B are free, one breed step");
    let step = plan.steps.last().expect("final step");
    assert_eq!(step.child, "C");
    let got = [step.pair.a.as_str(), step.pair.b.as_str()];
    assert!(got == ["A", "B"] || got == ["B", "A"], "chose {got:?}");
    assert!(step.ready, "opposite-gender A + B are ready");
    assert!(plan.leaves_to_obtain.is_empty());
}

#[test]
fn owned_target_single_gender_self_pair_only_is_pending() {
    // No alternative recipe: show the self-pair, flagged not-ready.
    let index = index(&[("C", &[("C", "C")])]);
    let owned = vec![pal("C", Gender::Male)];
    let base = base_costs(&[("C", 50)]);

    let plan = index.plan(&owned, "C", &base).expect("owned");

    assert_eq!(plan.total_cost, 1);
    let step = plan.steps.first().expect("one step");
    assert_eq!([step.pair.a.as_str(), step.pair.b.as_str()], ["C", "C"]);
    assert!(!step.ready, "single gender can't self-breed yet");
    assert!(plan.leaves_to_obtain.is_empty());
}
