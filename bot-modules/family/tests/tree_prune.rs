//! Collapse behaviour when a component exceeds a tier's node budget.
//!
//! The contract these pin: a component that fits is returned untouched (so an
//! ordinary free server never sees a collapsed tree), and a component that
//! does not fit is cut around the focus in a way that is reproducible run to
//! run -- an image that reshuffles between identical invocations reads as a
//! bug to the user.

use family::TreeQuota;
use family::tree::{RawGraph, RawPerson, prune};

/// A chain of `count` people, each the parent of the next, rooted at id 1.
fn lineage(count: i64) -> RawGraph {
    RawGraph {
        people: (1..=count)
            .map(|id| RawPerson { id, username: format!("user{id}") })
            .collect(),
        partners: Vec::new(),
        parents: (1..count).map(|id| (id, id + 1)).collect(),
        truncated: false,
    }
}

/// One parent with `count` children, so the focus has a very wide fan-out.
fn brood(count: i64) -> RawGraph {
    RawGraph {
        people: (1..=count)
            .map(|id| RawPerson { id, username: format!("user{id}") })
            .collect(),
        partners: Vec::new(),
        parents: (2..=count).map(|id| (1, id)).collect(),
        truncated: false,
    }
}

#[test]
fn a_component_within_budget_is_returned_untouched() {
    let raw = brood(10);
    let pruned = prune(raw, 1, TreeQuota::FREE);

    assert_eq!(pruned.shown(), 10);
    assert_eq!(pruned.total, 10);
    assert!(!pruned.is_collapsed(), "nothing was cut, so nothing to announce");
    assert!(pruned.hidden.is_empty(), "no chips on an uncut tree");
}

#[test]
fn a_component_over_budget_is_cut_to_the_budget() {
    let raw = brood(200);
    let pruned = prune(raw, 1, TreeQuota::FREE);

    assert!(
        pruned.shown() <= TreeQuota::FREE.node_budget,
        "shown {} exceeds the free budget",
        pruned.shown(),
    );
    assert_eq!(pruned.total, 200);
    assert!(pruned.is_collapsed());
}

#[test]
fn the_focus_always_survives_the_cut() {
    let raw = brood(200);
    let pruned = prune(raw, 1, TreeQuota::FREE);

    assert!(
        pruned.raw.people.iter().any(|p| p.id == 1),
        "the focus is the whole point of the picture",
    );
}

/// The generation cap bites before the node budget on a deep lineage: a
/// 40-person chain is well inside the free budget of 60, but only the focus's
/// own few generations should be drawn.
#[test]
fn the_generation_cap_bounds_a_deep_lineage() {
    let raw = lineage(200);
    let focus = 100;
    let pruned = prune(raw, focus, TreeQuota::FREE);

    let span = i64::from(TreeQuota::FREE.generation_span);
    for person in &pruned.raw.people {
        assert!(
            (person.id - focus).abs() <= span,
            "user {} is {} generations from the focus, cap is {span}",
            person.id,
            (person.id - focus).abs(),
        );
    }
}

/// A wider generation span is the visible half of what a higher tier buys, so
/// the same lineage must reach further at Ultra than at Free.
#[test]
fn a_higher_tier_reaches_further_through_the_same_graph() {
    let free = prune(lineage(200), 100, TreeQuota::FREE);
    let ultra = prune(lineage(200), 100, TreeQuota::ULTRA);

    assert!(
        ultra.shown() > free.shown(),
        "ultra showed {} but free showed {}",
        ultra.shown(),
        free.shown(),
    );
}

/// A graph that collapses on the free tier should render whole on Ultra --
/// that is the premium proposition stated as a test.
#[test]
fn a_graph_that_collapses_on_free_can_render_whole_on_ultra() {
    let people = 120;

    let free = prune(brood(people), 1, TreeQuota::FREE);
    let ultra = prune(brood(people), 1, TreeQuota::ULTRA);

    assert!(free.is_collapsed(), "120 people should not fit the free budget");
    assert!(!ultra.is_collapsed(), "120 people should fit the ultra budget");
    assert_eq!(
        ultra.shown(),
        usize::try_from(people).expect("count fits usize"),
    );
}

#[test]
fn dropped_neighbours_are_counted_against_the_node_that_kept_them() {
    let raw = brood(200);
    let pruned = prune(raw, 1, TreeQuota::FREE);

    let hidden = pruned.hidden.get(&1).copied().unwrap_or(0);
    let kept_children = pruned.shown() - 1;
    let expected =
        u32::try_from(199 - kept_children).expect("count fits u32");

    assert_eq!(
        hidden, expected,
        "the parent should account for every child left out",
    );
}

#[test]
fn surviving_edges_never_dangle() {
    let pruned = prune(brood(200), 1, TreeQuota::FREE);

    let ids: Vec<i64> = pruned.raw.people.iter().map(|p| p.id).collect();

    for (parent, child) in &pruned.raw.parents {
        assert!(ids.contains(parent), "edge parent {parent} was cut");
        assert!(ids.contains(child), "edge child {child} was cut");
    }
    for (a, b) in &pruned.raw.partners {
        assert!(ids.contains(a) && ids.contains(b), "partner edge dangles");
    }
}

/// Determinism is what stops the same command producing a different-looking
/// picture on every invocation.
#[test]
fn pruning_is_reproducible() {
    let first = prune(brood(200), 1, TreeQuota::FREE);
    let second = prune(brood(200), 1, TreeQuota::FREE);

    let ids = |p: &family::tree::Pruned| -> Vec<i64> {
        p.raw.people.iter().map(|person| person.id).collect()
    };

    assert_eq!(ids(&first), ids(&second));
    assert_eq!(first.raw.parents, second.raw.parents);
}
