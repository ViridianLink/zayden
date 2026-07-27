//! Slot / alternate accounting for an LFG post.
//!
//! Audit finding lfg #2 (`design-docs/audits/lfg.md`): the post lifecycle
//! shipped with no coverage of the capacity arithmetic — the logic lfg DS-1
//! (the fireteam capacity race) already broke once. `PostRow::join` decides
//! whether to roll the join transaction back purely on
//! `fireteam_len() > Join::fireteam_size()`, so the semantics pinned here are
//! the ones that guard actually depends on.

use jiff::Timestamp;
use jiff_sqlx::ToSqlx;
use lfg::{Join, PostRow};
use serenity::all::UserId;
use zayden_core::as_i64;

const OWNER: u64 = 211_486_447_369_322_496;
const THREAD: u64 = 1_099_425_082_890_113_024;

/// A post with an explicit roster — `PostBuilder` can only ever seed the owner,
/// so multi-member states are built from the row directly (as the DB returns
/// them).
fn post(fireteam: &[u64], alternatives: &[u64], fireteam_size: i16) -> PostRow {
    PostRow {
        id: as_i64(THREAD),
        owner_id: as_i64(OWNER),
        activity: "Vault of Glass".to_string(),
        start_time: Timestamp::UNIX_EPOCH.to_sqlx(),
        description: "Fresh run".to_string(),
        fireteam_size,
        fireteam: fireteam.iter().copied().map(as_i64).collect(),
        alternatives: alternatives.iter().copied().map(as_i64).collect(),
        alt_channel: None,
        alt_message: None,
    }
}

fn members(count: u64) -> Vec<u64> {
    (0..count).map(|i| OWNER + i).collect()
}

#[test]
fn fireteam_len_counts_only_the_fireteam() {
    let row = post(&members(3), &members(4), 6);

    assert_eq!(row.fireteam_len(), 3);
    assert_eq!(Join::fireteam_size(&row), 6);
}

#[test]
fn alternatives_never_consume_a_fireteam_slot() {
    // Four alternates on a three-strong fireteam must leave the post joinable;
    // if alternates ever counted toward capacity the post would read as full.
    let row = post(&members(3), &members(4), 6);

    assert!(!row.is_full(), "alternates must not fill the fireteam");
    assert_eq!(row.alternatives().count(), 4);
}

#[test]
fn is_full_flips_exactly_at_capacity() {
    assert!(!post(&members(5), &[], 6).is_full(), "5/6 is joinable");
    assert!(post(&members(6), &[], 6).is_full(), "6/6 is full");
}

#[test]
fn is_full_stays_true_past_capacity() {
    // An over-capacity row is what the DS-1 race produced; it must still read
    // as full rather than wrapping back to joinable.
    assert!(post(&members(7), &[], 6).is_full());
}

#[test]
fn join_rollback_predicate_admits_the_last_slot() {
    // `PostRow::join` rolls back on `fireteam_len() > fireteam_size()`. The
    // join that *fills* the final slot must therefore commit, and only the one
    // past it must fail — an off-by-one here either drops a legitimate join or
    // lets the post overfill.
    let filled = post(&members(6), &[], 6);
    assert!(filled.fireteam_len() <= Join::fireteam_size(&filled));

    let overfilled = post(&members(7), &[], 6);
    assert!(overfilled.fireteam_len() > Join::fireteam_size(&overfilled));
}

#[test]
fn single_slot_post_is_full_with_just_the_owner() {
    let row = post(&[OWNER], &[], 1);

    assert!(row.is_full());
    assert_eq!(row.fireteam_len(), 1);
}

#[test]
fn an_emptied_fireteam_is_never_full() {
    // Reachable today: every member (owner included) can leave without the post
    // being deleted, so the capacity check must not treat 0 members as full.
    let row = post(&[], &[], 6);

    assert_eq!(row.fireteam_len(), 0);
    assert!(!row.is_full());
}

#[test]
fn fireteam_ids_round_trip_to_user_ids() {
    let ids = members(3);
    let row = post(&ids, &[], 6);

    let seen = row.fireteam().collect::<Vec<_>>();
    let expected = ids.iter().copied().map(UserId::new).collect::<Vec<_>>();

    assert_eq!(seen, expected);
}

#[test]
fn fireteam_len_saturates_instead_of_wrapping() {
    // `fireteam_len` is an `i16::try_from(len).unwrap_or(i16::MAX)`. A roster
    // longer than `i16::MAX` is not reachable in practice, but the saturation
    // must clamp high (still "full") rather than wrap negative (joinable).
    let huge = usize::try_from(i16::MAX).unwrap_or(usize::MAX) + 1;
    let row = post(&vec![OWNER; huge], &[], 6);

    assert_eq!(row.fireteam_len(), i16::MAX);
    assert!(row.is_full());
}
