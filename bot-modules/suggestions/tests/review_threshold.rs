//! Regression coverage for the suggestions promote/demote thresholds.
//!
//! **DS-1:** the demote branch once used `neg_count - pos_count <= 15`,
//! inverting the sign of the delta so a heavily downvoted suggestion was never
//! removed from the review channel, while suggestions in the hysteresis gap
//! were spuriously demoted. `review_action` measures both thresholds against
//! the same signed `pos_count - neg_count` delta.
//!
//! **#3 (CC-8):** the two bounds were then `const` literals (20 / 15) with no
//! per-guild column and no dashboard field, so a guild whose suggestions never
//! reach +20 net had a permanently empty review channel and no way to lower the
//! bar short of a redeploy. They are now [`ReviewThresholds`] read from
//! `suggestions_settings`, and this file pins both the tuned behaviour and the
//! hysteresis invariant the web editor must not be able to violate.

use suggestions::{ReviewAction, ReviewThresholds, review_action};

fn defaults() -> ReviewThresholds {
    ReviewThresholds::default()
}

#[test]
fn defaults_match_the_previously_hardcoded_bounds() {
    // An unconfigured guild must behave exactly as it did before the columns
    // existed, and the migration's column defaults must agree with these.
    assert_eq!(ReviewThresholds::DEFAULT_PROMOTE, 20);
    assert_eq!(ReviewThresholds::DEFAULT_DEMOTE, 15);
    assert_eq!(defaults().promote(), 20);
    assert_eq!(defaults().demote(), 15);
}

#[test]
fn heavily_downvoted_post_is_demoted() {
    // delta = 5 - 25 = -20. Intended: delete the review post.
    // Pre-fix (`neg - pos = 20 <= 15` → false) this landed in the no-op gap and
    // the stale post lingered forever — the exact DS-1 failure scenario.
    assert_eq!(review_action(5, 25, defaults()), ReviewAction::Demote);
}

#[test]
fn promotion_threshold_is_inclusive_at_20() {
    assert_eq!(review_action(25, 0, defaults()), ReviewAction::Promote);
    assert_eq!(review_action(20, 0, defaults()), ReviewAction::Promote); // boundary
}

#[test]
fn hysteresis_gap_leaves_post_unchanged() {
    // delta in [16, 19] is neither promote nor demote.
    // Pre-fix (`neg - pos <= 15` → e.g. -19 <= 15 → true) this spuriously demoted
    // a post that should persist.
    assert_eq!(review_action(19, 0, defaults()), ReviewAction::Unchanged);
    assert_eq!(review_action(16, 0, defaults()), ReviewAction::Unchanged);
}

#[test]
fn demote_threshold_is_inclusive_at_15() {
    assert_eq!(review_action(15, 0, defaults()), ReviewAction::Demote); // boundary
    assert_eq!(review_action(0, 0, defaults()), ReviewAction::Demote);
    assert_eq!(review_action(3, 20, defaults()), ReviewAction::Demote); // delta -17
}

#[test]
fn small_guild_can_lower_the_bar_it_could_never_reach() {
    // The #3 failure scenario: a 50-member guild never reaches +20 net, so under
    // the hard-coded bounds every suggestion sat in Unchanged/Demote forever and
    // the review channel stayed empty. With promote=4 / demote=-2 the same
    // +5-net suggestion is promoted.
    let tuned = ReviewThresholds::new(4, -2);

    assert_eq!(review_action(5, 0, tuned), ReviewAction::Promote);
    assert_eq!(review_action(5, 0, defaults()), ReviewAction::Demote);

    // …and the tuned gap and demote bound move with it.
    assert_eq!(review_action(3, 0, tuned), ReviewAction::Unchanged);
    assert_eq!(review_action(1, 3, tuned), ReviewAction::Demote); // delta -2
}

#[test]
fn large_guild_can_raise_the_bar() {
    let tuned = ReviewThresholds::new(500, 100);

    assert_eq!(review_action(400, 0, tuned), ReviewAction::Unchanged);
    assert_eq!(review_action(500, 0, tuned), ReviewAction::Promote);
    assert_eq!(review_action(100, 0, tuned), ReviewAction::Demote);
}

#[test]
fn inverted_pair_is_normalised_instead_of_flapping() {
    // demote >= promote would make both branches true for the same delta, so a
    // review post would be created and deleted by alternating reactions. The
    // demote side yields to keep a one-wide gap.
    let bad = ReviewThresholds::new(10, 10);
    assert_eq!(bad.promote(), 10);
    assert_eq!(bad.demote(), 9);
    assert_eq!(review_action(10, 0, bad), ReviewAction::Promote);
    assert_eq!(review_action(9, 0, bad), ReviewAction::Demote);

    let worse = ReviewThresholds::new(5, 40);
    assert_eq!(worse.demote(), 4);
    assert!(worse.demote() < worse.promote());
}

#[test]
fn normalisation_does_not_overflow_at_the_bound() {
    let edge = ReviewThresholds::new(i32::MIN, i32::MAX);
    assert_eq!(edge.demote(), i32::MIN);
}

#[test]
fn parse_accepts_the_dashboard_form_fields() {
    let t = ReviewThresholds::parse(" 8 ", " -3 ");
    assert_eq!(t.promote(), 8);
    assert_eq!(t.demote(), -3);
}

#[test]
fn parse_falls_back_to_defaults_on_junk() {
    // An empty or non-numeric field must not zero the thresholds — that would
    // promote every suggestion at delta 0.
    let t = ReviewThresholds::parse("", "abc");
    assert_eq!(t.promote(), ReviewThresholds::DEFAULT_PROMOTE);
    assert_eq!(t.demote(), ReviewThresholds::DEFAULT_DEMOTE);
}

#[test]
fn parse_applies_the_hysteresis_invariant() {
    // The web editor cannot write a pair the bot would reject.
    let t = ReviewThresholds::parse("3", "3");
    assert!(t.demote() < t.promote());
}
