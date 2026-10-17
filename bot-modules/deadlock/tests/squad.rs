use deadlock::squad::{
    LEADERBOARD_LIMIT,
    LeaderboardEntry,
    RankChange,
    next_announcement,
    order_leaderboard,
    rank_change,
    streak_milestone,
};
use deadlock_api::rank::Badge;

macro_rules! badge {
    ($tier:expr, $subrank:expr) => {
        Badge::new($tier, $subrank).unwrap()
    };
}

const fn entry(
    discord_id: i64,
    badge: Option<Badge>,
    streak: i32,
) -> LeaderboardEntry {
    LeaderboardEntry { discord_id, badge, streak }
}

fn ids(entries: &[LeaderboardEntry]) -> Vec<i64> {
    entries.iter().map(|entry| entry.discord_id).collect()
}

#[test]
fn a_player_never_polled_is_seeded() {
    assert_eq!(rank_change(None, badge!(5, 3)), RankChange::Seeded);
}

#[test]
fn an_unranked_reading_never_changes_anything() {
    assert_eq!(rank_change(None, Badge::UNRANKED), RankChange::Unchanged);
    assert_eq!(rank_change(Some(53), Badge::UNRANKED), RankChange::Unchanged);
}

#[test]
fn the_same_badge_is_unchanged() {
    assert_eq!(rank_change(Some(53), badge!(5, 3)), RankChange::Unchanged);
}

#[test]
fn a_higher_badge_is_a_promotion() {
    assert_eq!(rank_change(Some(53), badge!(6, 1)), RankChange::Promoted {
        from: badge!(5, 3),
        to: badge!(6, 1)
    });
}

#[test]
fn a_lower_badge_is_a_demotion() {
    assert_eq!(rank_change(Some(53), badge!(5, 2)), RankChange::Demoted {
        from: badge!(5, 3),
        to: badge!(5, 2)
    });
}

#[test]
fn a_stored_badge_that_is_not_valid_counts_as_never_polled() {
    assert_eq!(rank_change(Some(99), badge!(5, 3)), RankChange::Seeded);
    assert_eq!(rank_change(Some(-4), badge!(5, 3)), RankChange::Seeded);
}

#[test]
fn milestones_round_down_and_keep_their_sign() {
    assert_eq!(streak_milestone(2), 0);
    assert_eq!(streak_milestone(3), 3);
    assert_eq!(streak_milestone(4), 3);
    assert_eq!(streak_milestone(5), 5);
    assert_eq!(streak_milestone(12), 10);
    assert_eq!(streak_milestone(-3), -3);
    assert_eq!(streak_milestone(-11), -10);
    assert_eq!(streak_milestone(0), 0);
}

#[test]
fn a_milestone_is_announced_once() {
    assert_eq!(next_announcement(0, 3), (3, Some(3)));
    assert_eq!(next_announcement(3, 4), (3, None));
    assert_eq!(next_announcement(3, 5), (5, Some(5)));
}

#[test]
fn a_broken_streak_rearms_the_announcement() {
    assert_eq!(next_announcement(5, 1), (0, None));
    assert_eq!(next_announcement(5, -3), (-3, Some(-3)));
}

#[test]
fn the_leaderboard_orders_by_badge_then_streak_then_id() {
    let mut entries = vec![
        entry(1, Some(badge!(4, 1)), 9),
        entry(2, Some(badge!(6, 2)), 0),
        entry(4, Some(badge!(4, 1)), 9),
        entry(3, Some(badge!(4, 1)), 2),
    ];

    order_leaderboard(&mut entries);

    assert_eq!(ids(&entries), [2, 1, 4, 3]);
}

#[test]
fn unranked_and_unseen_players_sort_last() {
    let mut entries = vec![
        entry(1, None, 12),
        entry(2, Some(Badge::UNRANKED), 3),
        entry(3, Some(badge!(1, 1)), -4),
    ];

    order_leaderboard(&mut entries);

    assert_eq!(ids(&entries), [3, 1, 2]);
}

#[test]
fn the_leaderboard_is_cut_to_its_limit() {
    let mut entries: Vec<LeaderboardEntry> =
        (0..30).map(|id| entry(id, Some(badge!(3, 1)), 0)).collect();

    order_leaderboard(&mut entries);

    assert_eq!(entries.len(), LEADERBOARD_LIMIT);
    assert_eq!(entries.first().map(|entry| entry.discord_id), Some(0));
}
