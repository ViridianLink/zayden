//! Guards the daily-goal registry against id collisions.
//!
//! `WIN_10` was declared as `GoalDefinition::new("gift")`, the same id as
//! `GIFT`. `GoalRegistry` is a `HashMap` keyed on that id, so one of the two
//! definitions was silently dropped on construction: the registry held eight of
//! the nine declared goals, and which one survived depended on iteration order.
//! Nothing failed loudly — the goal simply never appeared.

use std::collections::HashSet;

use gambling::goals::GOAL_REGISTRY;

/// Every goal declared in `definitions.rs`. A collision shows up here as a
/// missing id rather than as a silently shorter registry.
const DECLARED_GOALS: [&str; 9] = [
    "lotto",
    "gift",
    "win_10",
    "higherlower",
    "winmaxbet",
    "win3row",
    "allin",
    "sendcoins",
    "work",
];

#[test]
fn every_declared_goal_survives_registry_construction() {
    for id in DECLARED_GOALS {
        assert!(
            GOAL_REGISTRY.get_definition(id).is_some(),
            "goal '{id}' is missing from the registry — most likely its id \
             collides with another declaration and the HashMap dropped it"
        );
    }
}

#[test]
fn declared_goal_ids_are_unique() {
    let unique = DECLARED_GOALS.iter().collect::<HashSet<_>>().len();

    assert_eq!(
        unique,
        DECLARED_GOALS.len(),
        "two goals share an id, so the registry will drop one"
    );
}
