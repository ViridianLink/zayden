mod common;

use common::{Mock, Reply, api, spawn_mock};
use deadlock::db::players;
use deadlock::match_poll::{self, PollOutcome};
use jiff::Timestamp;
use sqlx::PgPool;

const METADATA: &str = include_str!("fixtures/metadata_bulk.json");
const ITEMS: &str = include_str!("fixtures/items.json");
const HEROES: &str = include_str!("fixtures/heroes.json");

const STEAM_ID: &str = "76561198101876716";
const OTHER_STEAM_ID: &str = "76561198000000001";
const ACCOUNT: u32 = 141_610_988;
const OTHER_ACCOUNT: u32 = 39_734_273;

macro_rules! mock {
    () => {{
        let mock = spawn_mock(Reply::ok("[]")).await.unwrap();

        mock.serve("/v1/matches/metadata", Reply::ok(METADATA));
        mock.serve("/v1/assets/items", Reply::ok(ITEMS));
        mock.serve("/v1/assets/heroes", Reply::ok(HEROES));

        mock
    }};
}

macro_rules! link {
    ($pool:expr, $discord_id:expr, $steam_id:expr) => {
        sqlx::query!(
            "INSERT INTO account_links (discord_id, provider, external_id) VALUES ($1, 'steam', $2)",
            $discord_id,
            $steam_id
        )
        .execute($pool)
        .await
        .unwrap()
    };
}

macro_rules! seeded_before_the_fixture_matches {
    ($pool:expr) => {
        sqlx::query!(
            "UPDATE deadlock_players SET seeded_at = '2026-01-01T00:00:00Z'"
        )
        .execute($pool)
        .await
        .unwrap()
    };
}

macro_rules! seen_rows {
    ($pool:expr) => {
        sqlx::query_scalar!(
            r#"SELECT count(*) AS "count!" FROM deadlock_matches_seen"#
        )
        .fetch_one($pool)
        .await
        .unwrap()
    };
}

fn request_count(mock: &Mock) -> usize {
    mock.requests.lock().map_or(0, |recorded| recorded.len())
}

#[sqlx::test(migrations = "../../migrations")]
async fn the_first_poll_summarises_new_matches_and_the_second_finds_none(
    pool: PgPool,
) {
    let mock = mock!();
    let api = api(&pool, &mock.base).unwrap();
    link!(&pool, 1, STEAM_ID);
    players::sync(&pool).await.unwrap();
    seeded_before_the_fixture_matches!(&pool);

    let (outcome, jobs) =
        match_poll::poll(&api, &pool, Timestamp::now()).await.unwrap();

    assert_eq!(outcome, PollOutcome { players: 1, matches: 2, new: 2 });
    assert_eq!(jobs.len(), 2);
    assert!(jobs.iter().all(|job| job.discord_id == 1));
    assert_eq!(jobs[0].summary.match_id, 69_192_735);
    assert_eq!(jobs[1].summary.match_id, 69_196_035);
    assert_eq!(players::all(&pool).await.unwrap()[0].streak, 2);
    assert_eq!(seen_rows!(&pool), 2);

    let (again, none) =
        match_poll::poll(&api, &pool, Timestamp::now()).await.unwrap();

    assert_eq!(again, PollOutcome { players: 1, matches: 2, new: 0 });
    assert!(none.is_empty());
    assert_eq!(players::all(&pool).await.unwrap()[0].streak, 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn matches_from_before_the_link_are_not_backfilled(pool: PgPool) {
    let mock = mock!();
    let api = api(&pool, &mock.base).unwrap();
    link!(&pool, 1, STEAM_ID);

    let (outcome, jobs) =
        match_poll::poll(&api, &pool, Timestamp::now()).await.unwrap();

    assert_eq!(outcome, PollOutcome { players: 1, matches: 2, new: 0 });
    assert!(jobs.is_empty());
    assert_eq!(seen_rows!(&pool), 0);
    assert_eq!(players::all(&pool).await.unwrap()[0].streak, 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn without_linked_players_nothing_is_requested(pool: PgPool) {
    let mock = mock!();
    let api = api(&pool, &mock.base).unwrap();

    let (outcome, jobs) =
        match_poll::poll(&api, &pool, Timestamp::now()).await.unwrap();

    assert_eq!(outcome, PollOutcome::default());
    assert!(jobs.is_empty());
    assert_eq!(request_count(&mock), 0);
}

#[sqlx::test(migrations = "../../migrations")]
async fn relinking_to_another_steam_account_replaces_the_player(pool: PgPool) {
    link!(&pool, 1, STEAM_ID);
    players::sync(&pool).await.unwrap();

    let before = players::all(&pool).await.unwrap();
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].account_id, ACCOUNT);

    sqlx::query!(
        "UPDATE account_links SET external_id = $1 WHERE discord_id = 1",
        OTHER_STEAM_ID
    )
    .execute(&pool)
    .await
    .unwrap();
    players::sync(&pool).await.unwrap();

    let after = players::all(&pool).await.unwrap();
    assert_eq!(after.len(), 1);
    assert_eq!(after[0].account_id, OTHER_ACCOUNT);
    assert_eq!(after[0].discord_id, 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn unlinking_removes_the_player_and_its_seen_matches(pool: PgPool) {
    let mock = mock!();
    let api = api(&pool, &mock.base).unwrap();
    link!(&pool, 1, STEAM_ID);
    players::sync(&pool).await.unwrap();
    seeded_before_the_fixture_matches!(&pool);
    match_poll::poll(&api, &pool, Timestamp::now()).await.unwrap();
    assert_eq!(seen_rows!(&pool), 2);

    sqlx::query!("DELETE FROM account_links WHERE discord_id = 1")
        .execute(&pool)
        .await
        .unwrap();

    assert!(players::all(&pool).await.unwrap().is_empty());
    assert_eq!(seen_rows!(&pool), 0);
}
