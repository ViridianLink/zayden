//! Graph-shape coverage: partner blocks, unions, and generation assignment.
//!
//! The schema is a graph, not a tree -- `family_partners` allows a
//! configurable number of partners, `family_parent_child` allows several
//! parents per child and only forbids self-loops. These tests pin the cases
//! that a tree-shaped assumption would get wrong.

use std::collections::HashMap;

use family::tree::layout::layout;
use family::tree::model::FamilyGraph;
use family::tree::{RawGraph, RawPerson};

/// A typed empty edge list; `[]` alone cannot infer its element type here.
const NONE: [(i64, i64); 0] = [];

/// Builds a component directly, bypassing the database.
macro_rules! graph {
    (people: $people:expr, partners: $partners:expr, parents: $parents:expr, focus: $focus:expr $(,)?) => {{
        let mut people: Vec<RawPerson> = $people
            .iter()
            .map(|id: &i64| RawPerson {
                id: *id,
                username: format!("user{id}"),
            })
            .collect();
        people.sort_by_key(|p| p.id);

        let raw = RawGraph {
            people,
            partners: $partners.to_vec(),
            parents: $parents.to_vec(),
            truncated: false,
        };

        FamilyGraph::from_raw(&raw, $focus, &HashMap::new())
            .expect("focus should be present in the component")
    }};
}

/// Resolves a person id to its node index.
macro_rules! node {
    ($graph:expr, $id:expr) => {
        $graph
            .people
            .iter()
            .position(|p| p.id == $id)
            .expect("person should be present")
    };
}

#[test]
fn a_married_couple_forms_one_block() {
    let graph = graph!(
        people: [1i64, 2],
        partners: [(1i64, 2)],
        parents: NONE,
        focus: 1,
    );

    assert_eq!(graph.blocks.len(), 1);
    assert_eq!(graph.blocks.first().map(|b| b.members.len()), Some(2));
}

/// `family_settings.max_partners` is configurable, so three mutually married
/// people are a legal state and must land in a single block rather than being
/// split across the canvas.
#[test]
fn a_polygamous_group_forms_a_single_block() {
    let graph = graph!(
        people: [1i64, 2, 3],
        partners: [(1i64, 2), (1, 3), (2, 3)],
        parents: NONE,
        focus: 1,
    );

    assert_eq!(graph.blocks.len(), 1, "one partner group, one block");
    assert_eq!(graph.blocks.first().map(|b| b.members.len()), Some(3));
}

/// Partner edges are transitive through the group even when not every pair is
/// recorded: 1-2 and 2-3 still means one household.
#[test]
fn partner_groups_are_transitive() {
    let graph = graph!(
        people: [1i64, 2, 3],
        partners: [(1i64, 2), (2, 3)],
        parents: NONE,
        focus: 1,
    );

    assert_eq!(graph.blocks.len(), 1);
}

#[test]
fn a_single_parent_produces_a_one_parent_union() {
    let graph = graph!(
        people: [1i64, 2],
        partners: NONE,
        parents: [(1i64, 2)],
        focus: 1,
    );

    assert_eq!(graph.unions.len(), 1);
    assert_eq!(graph.unions.first().map(|u| u.parents.len()), Some(1));
    assert_eq!(graph.unions.first().map(|u| u.children.len()), Some(1));
}

/// Two people who share a child without being partners still parent that child
/// jointly. Keying unions on the parent *set* handles this with no special
/// case, and the two stay in separate blocks because they are not married.
#[test]
fn unmarried_co_parents_share_one_union_but_not_a_block() {
    let graph = graph!(
        people: [1i64, 2, 3],
        partners: NONE,
        parents: [(1i64, 3), (2, 3)],
        focus: 1,
    );

    assert_eq!(graph.unions.len(), 1, "one child, one parent set");
    assert_eq!(graph.unions.first().map(|u| u.parents.len()), Some(2));
    assert_eq!(graph.blocks.len(), 3, "co-parents are not partners");
}

/// A person with children by two different partners produces two unions, so
/// each set of children hangs off the right pairing.
#[test]
fn children_by_different_partners_produce_separate_unions() {
    let graph = graph!(
        people: [1i64, 2, 3, 4, 5],
        partners: [(1i64, 2), (1, 3)],
        parents: [(1i64, 4), (2, 4), (1, 5), (3, 5)],
        focus: 1,
    );

    assert_eq!(graph.unions.len(), 2, "two distinct parent sets");

    let sizes: Vec<usize> =
        graph.unions.iter().map(|u| u.children.len()).collect();
    assert_eq!(sizes, vec![1, 1]);
}

#[test]
fn siblings_of_one_couple_share_a_single_union() {
    let graph = graph!(
        people: [1i64, 2, 3, 4],
        partners: [(1i64, 2)],
        parents: [(1i64, 3), (2, 3), (1, 4), (2, 4)],
        focus: 1,
    );

    assert_eq!(graph.unions.len(), 1);
    assert_eq!(graph.unions.first().map(|u| u.children.clone()), {
        let (a, b) = (node!(graph, 3), node!(graph, 4));
        Some(vec![a.min(b), a.max(b)])
    });
}

#[test]
fn generations_run_from_parents_down_to_children() {
    let graph = graph!(
        people: [1i64, 2, 3],
        partners: NONE,
        parents: [(1i64, 2), (2, 3)],
        focus: 2,
    );

    let placed = layout(&graph);
    let generation = |id: i64| {
        placed.generation.get(node!(graph, id)).copied().unwrap_or(-1)
    };

    assert_eq!(generation(1), 0, "grandparent at the top");
    assert_eq!(generation(2), 1);
    assert_eq!(generation(3), 2);
    assert_eq!(placed.generations, 3);
}

#[test]
fn partners_always_share_a_generation() {
    // 3 is a child of 1, and married to 2 who has no parents. Without the
    // partner constraint 2 would float up to generation 0.
    let graph = graph!(
        people: [1i64, 2, 3],
        partners: [(2i64, 3)],
        parents: [(1i64, 3)],
        focus: 3,
    );

    let placed = layout(&graph);
    let generation = |id: i64| {
        placed.generation.get(node!(graph, id)).copied().unwrap_or(-1)
    };

    assert_eq!(generation(2), generation(3), "partners share a row");
    assert!(generation(1) < generation(3), "the parent stays above");
}

/// `family_parent_child` only forbids self-loops, so `A parent of B, B parent
/// of A` is storable. Generation assignment must terminate and mark the
/// offending edge rather than spinning or distorting every row.
#[test]
fn a_parent_cycle_terminates_and_is_marked_as_a_back_edge() {
    let graph = graph!(
        people: [1i64, 2],
        partners: NONE,
        parents: [(1i64, 2), (2, 1)],
        focus: 1,
    );

    let placed = layout(&graph);

    assert_eq!(placed.back_edges.len(), 1, "exactly one edge closes the cycle");
    assert_eq!(placed.generations, 2, "the rows stay sane");
}

/// A longer cycle exercises the DFS colouring rather than the trivial pair.
#[test]
fn a_three_node_parent_cycle_terminates() {
    let graph = graph!(
        people: [1i64, 2, 3],
        partners: NONE,
        parents: [(1i64, 2), (2, 3), (3, 1)],
        focus: 1,
    );

    let placed = layout(&graph);

    assert_eq!(placed.back_edges.len(), 1);
    assert!(placed.generations <= 3);
}

#[test]
fn from_raw_rejects_a_component_without_the_focus() {
    let raw = RawGraph {
        people: vec![RawPerson { id: 1, username: "a".to_string() }],
        partners: Vec::new(),
        parents: Vec::new(),
        truncated: false,
    };

    assert!(FamilyGraph::from_raw(&raw, 99, &HashMap::new()).is_none());
}
