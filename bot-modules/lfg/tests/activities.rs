//! The activity catalog's own invariants.

use lfg::{ACTIVITIES, ActivityCategory};

#[test]
fn every_activity_has_a_usable_fireteam_size() {
    // `fireteam_size` seeds the post's capacity; a zero or negative default
    // would make the post full on creation.
    for activity in &ACTIVITIES {
        assert!(
            activity.fireteam_size > 0,
            "`{}` has a non-positive fireteam size",
            activity.name,
        );
        assert!(
            activity.fireteam_size <= 6,
            "`{}` exceeds Destiny's six-guardian maximum",
            activity.name,
        );
    }
}

#[test]
fn activity_names_are_unique() {
    // Names are the autocomplete/choice key, so duplicates are unresolvable.
    let mut names = ACTIVITIES.iter().map(|a| a.name).collect::<Vec<_>>();
    let before = names.len();
    names.sort_unstable();
    names.dedup();

    assert_eq!(names.len(), before, "duplicate activity name in the catalog");
}

#[test]
fn raids_and_dungeons_carry_their_canonical_sizes() {
    for activity in &ACTIVITIES {
        match activity.category {
            ActivityCategory::Raid => assert_eq!(
                activity.fireteam_size, 6,
                "raid `{}` is not a six-stack",
                activity.name,
            ),
            ActivityCategory::Dungeon => assert_eq!(
                activity.fireteam_size, 3,
                "dungeon `{}` is not a three-stack",
                activity.name,
            ),
            ActivityCategory::ExoticMission
            | ActivityCategory::Vanguard
            | ActivityCategory::Pvp => {},
        }
    }
}
