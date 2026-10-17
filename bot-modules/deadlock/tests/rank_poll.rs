mod common;

use common::{Mock, Reply, api, spawn_mock};
use deadlock::db::{guild_members, players};
use deadlock::rank_poll::{self, SquadEventKind};
use deadlock_api::rank::Badge;
use jiff::Timestamp;
use sqlx::PgPool;

const STEAM_ID: &str = "76561198101876716";
const ACCOUNT: u32 = 141_610_988;
const GUILD: i64 = 7;
const MEMBER: i64 = 1;

const BADGE_53: &str = r#"[{"account_id":141610988,"badge":53,"rank":5,"subrank":3,"last_match":null}]"#;
const BADGE_61: &str = r#"[{"account_id":141610988,"badge":61,"rank":6,"subrank":1,"last_match":null}]"#;

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

macro_rules! snapshot_rows {
    ($pool:expr) => {
        sqlx::query_scalar!(
            r#"SELECT count(*) AS "count!" FROM deadlock_rank_snapshots"#
        )
        .fetch_one($pool)
        .await
        .unwrap()
    };
}

fn request_count(mock: &Mock) -> usize {
    mock.requests.lock().map_or(0, |recorded| recorded.len())
}

macro_rules! at {
    ($second:expr) => {
        Timestamp::from_second($second).unwrap()
    };
}

macro_rules! badge {
    ($tier:expr, $subrank:expr) => {
        Badge::new($tier, $subrank).unwrap()
    };
}

#[sqlx::test(migrations = "../../migrations")]
async fn the_first_poll_seeds_the_badge_and_a_higher_one_ranks_up(pool: PgPool) {
    let mock = spawn_mock(Reply::ok("[]")).await.unwrap();
    mock.serve("/v1/players/rank", Reply::ok(BADGE_53));
    let api = api(&pool, &mock.base).unwrap();
    link!(&pool, MEMBER, STEAM_ID);
    guild_members::set_squad(&pool, GUILD, MEMBER, true).await.unwrap();

    let (outcome, events) = rank_poll::poll(&api, &pool, at!(1_000)).await.unwrap();

    assert_eq!(outcome.players, 1);
    assert_eq!(outcome.changed, 0);
    assert!(events.is_empty());
    assert_eq!(players::squad_players(&pool).await.unwrap()[0].badge, Some(53));
    assert_eq!(snapshot_rows!(&pool), 1);

    mock.serve("/v1/players/rank", Reply::ok(BADGE_61));
    let (outcome, events) = rank_poll::poll(&api, &pool, at!(2_000)).await.unwrap();

    assert_eq!(outcome.changed, 1);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].discord_id, MEMBER);
    assert_eq!(events[0].kind, SquadEventKind::RankUp {
        from: badge!(5, 3),
        to: badge!(6, 1)
    });
    assert_eq!(players::squad_players(&pool).await.unwrap()[0].badge, Some(61));
    assert_eq!(snapshot_rows!(&pool), 2);

    let (_, again) = rank_poll::poll(&api, &pool, at!(3_000)).await.unwrap();

    assert!(again.is_empty());
    assert_eq!(snapshot_rows!(&pool), 2);
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_streak_milestone_is_announced_once(pool: PgPool) {
    let mock = spawn_mock(Reply::ok("[]")).await.unwrap();
    let api = api(&pool, &mock.base).unwrap();
    link!(&pool, MEMBER, STEAM_ID);
    guild_members::set_squad(&pool, GUILD, MEMBER, true).await.unwrap();
    players::sync(&pool).await.unwrap();
    players::set_streak(&pool, ACCOUNT, 3).await.unwrap();

    let (_, events) = rank_poll::poll(&api, &pool, at!(1_000)).await.unwrap();

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, SquadEventKind::Streak(3));

    let (_, again) = rank_poll::poll(&api, &pool, at!(2_000)).await.unwrap();

    assert!(again.is_empty());
}

#[sqlx::test(migrations = "../../migrations")]
async fn players_outside_every_squad_are_not_requested(pool: PgPool) {
    let mock = spawn_mock(Reply::ok("[]")).await.unwrap();
    mock.serve("/v1/players/rank", Reply::ok(BADGE_53));
    let api = api(&pool, &mock.base).unwrap();
    link!(&pool, MEMBER, STEAM_ID);

    let (outcome, events) = rank_poll::poll(&api, &pool, at!(1_000)).await.unwrap();

    assert_eq!(outcome.players, 0);
    assert!(events.is_empty());
    assert_eq!(request_count(&mock), 0);
}
