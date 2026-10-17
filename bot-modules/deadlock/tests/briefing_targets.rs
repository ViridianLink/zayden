use deadlock::briefing::RECENT_WINDOW;
use deadlock::db::briefings::{RecentHero, claim, recent_heroes};
use deadlock::db::patches::{awaiting_briefing, mark_briefed};
use jiff::Timestamp;
use sqlx::PgPool;

macro_rules! at {
    ($value:expr) => {
        $value.parse::<Timestamp>().unwrap()
    };
}

#[sqlx::test(migrations = "../../migrations", fixtures("briefing_targets"))]
async fn only_opted_in_players_who_have_not_played_since_are_targeted(pool: PgPool) {
    let published = at!("2026-10-10T12:00:00Z");

    let rows = recent_heroes(&pool, "g1", published, published - RECENT_WINDOW)
        .await
        .unwrap();

    assert_eq!(rows, vec![RecentHero {
        discord_id: 10,
        hero_id: 1,
        games: 2,
        last_played: at!("2026-10-09T08:00:00Z"),
    }]);
}

#[sqlx::test(migrations = "../../migrations", fixtures("briefing_targets"))]
async fn a_briefing_is_claimed_once_per_player_and_patch(pool: PgPool) {
    assert!(claim(&pool, "g1", 10).await.unwrap());
    assert!(!claim(&pool, "g1", 10).await.unwrap());
    assert!(claim(&pool, "g1", 99).await.unwrap());
}

#[sqlx::test(migrations = "../../migrations", fixtures("briefing_targets"))]
async fn only_unbriefed_patches_with_a_diff_inside_the_window_await_briefing(
    pool: PgPool,
) {
    let awaiting =
        awaiting_briefing(&pool, at!("2026-10-09T00:00:00Z")).await.unwrap();
    let guids: Vec<&str> =
        awaiting.iter().map(|patch| patch.guid.as_str()).collect();

    assert_eq!(guids, vec!["g1"]);

    assert!(
        awaiting_briefing(&pool, at!("2026-10-11T00:00:00Z"))
            .await
            .unwrap()
            .is_empty()
    );

    mark_briefed(&pool, "g1").await.unwrap();

    assert!(
        awaiting_briefing(&pool, at!("2026-10-09T00:00:00Z"))
            .await
            .unwrap()
            .is_empty()
    );
}
