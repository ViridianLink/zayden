//! A guild's data is purged only once every bot sharing the database has been
//! gone from it for the whole retention window, and re-adding any of them
//! within the window cancels the purge.

use jiff::{SignedDuration, Timestamp};
use sqlx::PgPool;
use zayden_app::guilds::{
    GuildPresence,
    JoinKind,
    OwnPresence,
    RETENTION_DAYS,
    Shard,
};

const ZAYDEN: i64 = 100;
const VIKTOR: i64 = 200;

/// `(id >> 22) % 2` puts these on shards 0 and 1 respectively.
const SHARD_0_GUILD: i64 = 1;
const SHARD_1_GUILD: i64 = 1 << 22;

const fn joined() -> Timestamp {
    Timestamp::constant(1_750_000_000, 0)
}

fn later() -> Timestamp {
    joined() + SignedDuration::from_hours(24)
}

#[expect(
    clippy::expect_used,
    reason = "a free helper sits outside the #[test] items clippy.toml exempts"
)]
async fn mark_seeded(pool: &PgPool, guild_id: i64) {
    sqlx::query!("UPDATE guilds SET bot_joined_at = now() WHERE id = $1", guild_id)
        .execute(pool)
        .await
        .expect("the seed marker is written");
}

#[expect(
    clippy::expect_used,
    reason = "a free helper sits outside the #[test] items clippy.toml exempts"
)]
async fn backdate(pool: &PgPool, guild_id: i64, days: i32) {
    sqlx::query!(
        "UPDATE guild_presence SET left_at = now() - make_interval(days => $2)
         WHERE guild_id = $1 AND left_at IS NOT NULL",
        guild_id,
        days
    )
    .execute(pool)
    .await
    .expect("the backdate runs");
}

#[expect(
    clippy::expect_used,
    reason = "a free helper sits outside the #[test] items clippy.toml exempts"
)]
async fn guild_exists(pool: &PgPool, guild_id: i64) -> bool {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM guilds WHERE id = $1) AS "exists!""#,
        guild_id
    )
    .fetch_one(pool)
    .await
    .expect("the guild lookup runs")
}

#[expect(
    clippy::expect_used,
    reason = "a free helper sits outside the #[test] items clippy.toml exempts"
)]
async fn left_at_is_set(pool: &PgPool, guild_id: i64, application_id: i64) -> bool {
    sqlx::query_scalar!(
        r#"SELECT left_at IS NOT NULL AS "left!" FROM guild_presence
           WHERE guild_id = $1 AND application_id = $2"#,
        guild_id,
        application_id
    )
    .fetch_one(pool)
    .await
    .expect("the presence lookup runs")
}

#[sqlx::test(migrations = "../migrations")]
async fn a_guild_inside_the_window_is_kept(pool: PgPool) {
    GuildPresence::joined(&pool, SHARD_0_GUILD, ZAYDEN, joined()).await.unwrap();
    GuildPresence::left(&pool, SHARD_0_GUILD, ZAYDEN).await.unwrap();
    backdate(&pool, SHARD_0_GUILD, RETENTION_DAYS - 1).await;

    assert_eq!(
        GuildPresence::expired(&pool, RETENTION_DAYS).await.unwrap(),
        Vec::<i64>::new()
    );
    assert!(
        !GuildPresence::purge(&pool, SHARD_0_GUILD, RETENTION_DAYS).await.unwrap()
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_guild_past_the_window_is_purged_with_its_data(pool: PgPool) {
    GuildPresence::joined(&pool, SHARD_0_GUILD, ZAYDEN, joined()).await.unwrap();
    sqlx::query!("INSERT INTO music_settings (guild_id) VALUES ($1)", SHARD_0_GUILD)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query!("INSERT INTO guild_rules (guild_id) VALUES ($1)", SHARD_0_GUILD)
        .execute(&pool)
        .await
        .unwrap();

    GuildPresence::left(&pool, SHARD_0_GUILD, ZAYDEN).await.unwrap();
    backdate(&pool, SHARD_0_GUILD, RETENTION_DAYS + 1).await;

    assert_eq!(GuildPresence::expired(&pool, RETENTION_DAYS).await.unwrap(), [
        SHARD_0_GUILD
    ]);
    assert!(
        GuildPresence::purge(&pool, SHARD_0_GUILD, RETENTION_DAYS).await.unwrap()
    );
    assert!(!guild_exists(&pool, SHARD_0_GUILD).await);

    let leftovers = sqlx::query_scalar!(
        r#"SELECT (SELECT count(*) FROM music_settings WHERE guild_id = $1)
                + (SELECT count(*) FROM guild_rules WHERE guild_id = $1) AS "count!""#,
        SHARD_0_GUILD
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(leftovers, 0);
}

#[sqlx::test(migrations = "../migrations")]
async fn re_adding_the_bot_cancels_the_purge(pool: PgPool) {
    GuildPresence::joined(&pool, SHARD_0_GUILD, ZAYDEN, joined()).await.unwrap();
    GuildPresence::left(&pool, SHARD_0_GUILD, ZAYDEN).await.unwrap();
    backdate(&pool, SHARD_0_GUILD, RETENTION_DAYS + 1).await;

    GuildPresence::joined(&pool, SHARD_0_GUILD, ZAYDEN, joined()).await.unwrap();

    assert_eq!(
        GuildPresence::expired(&pool, RETENTION_DAYS).await.unwrap(),
        Vec::<i64>::new()
    );
    assert!(
        !GuildPresence::purge(&pool, SHARD_0_GUILD, RETENTION_DAYS).await.unwrap()
    );
    assert!(guild_exists(&pool, SHARD_0_GUILD).await);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_sibling_bot_still_present_keeps_the_guild(pool: PgPool) {
    GuildPresence::joined(&pool, SHARD_0_GUILD, ZAYDEN, joined()).await.unwrap();
    GuildPresence::joined(&pool, SHARD_0_GUILD, VIKTOR, joined()).await.unwrap();
    GuildPresence::left(&pool, SHARD_0_GUILD, ZAYDEN).await.unwrap();
    backdate(&pool, SHARD_0_GUILD, RETENTION_DAYS + 1).await;

    assert_eq!(
        GuildPresence::expired(&pool, RETENTION_DAYS).await.unwrap(),
        Vec::<i64>::new()
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_repeated_removal_keeps_the_first_departure_time(pool: PgPool) {
    GuildPresence::joined(&pool, SHARD_0_GUILD, ZAYDEN, joined()).await.unwrap();
    GuildPresence::left(&pool, SHARD_0_GUILD, ZAYDEN).await.unwrap();
    backdate(&pool, SHARD_0_GUILD, RETENTION_DAYS + 1).await;

    GuildPresence::left(&pool, SHARD_0_GUILD, ZAYDEN).await.unwrap();

    assert_eq!(GuildPresence::expired(&pool, RETENTION_DAYS).await.unwrap(), [
        SHARD_0_GUILD
    ]);
}

#[sqlx::test(migrations = "../migrations")]
async fn leaving_a_guild_with_no_data_records_nothing(pool: PgPool) {
    GuildPresence::left(&pool, SHARD_0_GUILD, ZAYDEN).await.unwrap();

    assert!(!guild_exists(&pool, SHARD_0_GUILD).await);
}

/// Ready only lists the guilds of its own shard, so guilds on other shards
/// must not be mistaken for removals.
#[sqlx::test(migrations = "../migrations")]
async fn reconcile_marks_only_missing_guilds_on_its_own_shard(pool: PgPool) {
    let listed = SHARD_0_GUILD + 2;
    for guild_id in [SHARD_0_GUILD, listed, SHARD_1_GUILD] {
        GuildPresence::joined(&pool, guild_id, ZAYDEN, joined()).await.unwrap();
    }

    let marked =
        GuildPresence::reconcile(&pool, ZAYDEN, Shard { id: 0, total: 2 }, &[
            listed,
        ])
        .await
        .unwrap();

    assert_eq!(marked, 1);
    assert!(left_at_is_set(&pool, SHARD_0_GUILD, ZAYDEN).await);
    assert!(!left_at_is_set(&pool, listed, ZAYDEN).await);
    assert!(!left_at_is_set(&pool, SHARD_1_GUILD, ZAYDEN).await);
}

/// A guild stored before presence tracking existed, and never seen by this bot
/// since, still enters the window instead of being kept forever.
#[sqlx::test(migrations = "../migrations")]
async fn reconcile_covers_guilds_stored_before_tracking(pool: PgPool) {
    sqlx::query!("INSERT INTO guilds (id) VALUES ($1)", SHARD_0_GUILD)
        .execute(&pool)
        .await
        .unwrap();

    GuildPresence::reconcile(&pool, ZAYDEN, Shard { id: 0, total: 1 }, &[])
        .await
        .unwrap();

    assert!(left_at_is_set(&pool, SHARD_0_GUILD, ZAYDEN).await);
}

#[test]
fn an_unseeded_guild_is_a_first_join() {
    assert_eq!(JoinKind::classify(false, None, false, joined()), JoinKind::First);
}

#[test]
fn the_same_membership_is_a_reconnect() {
    let own = OwnPresence { joined_at: Some(joined()), present: true };
    assert_eq!(
        JoinKind::classify(true, Some(own), true, joined()),
        JoinKind::Reconnect
    );
}

#[test]
fn a_row_from_before_joined_at_was_tracked_is_a_reconnect() {
    let own = OwnPresence { joined_at: None, present: true };
    assert_eq!(
        JoinKind::classify(true, Some(own), false, later()),
        JoinKind::Reconnect
    );
}

#[test]
fn joining_beside_a_present_bot_is_additional() {
    assert_eq!(JoinKind::classify(true, None, true, joined()), JoinKind::Additional);
}

#[test]
fn a_reconcile_placeholder_row_does_not_count_as_membership() {
    let own = OwnPresence { joined_at: None, present: false };
    assert_eq!(
        JoinKind::classify(true, Some(own), true, joined()),
        JoinKind::Additional
    );
}

#[test]
fn returning_after_every_bot_left_is_a_rejoin() {
    let own = OwnPresence { joined_at: Some(joined()), present: false };
    assert_eq!(
        JoinKind::classify(true, Some(own), false, later()),
        JoinKind::Rejoin
    );
}

#[test]
fn being_re_added_while_offline_is_a_rejoin() {
    let own = OwnPresence { joined_at: Some(joined()), present: true };
    assert_eq!(
        JoinKind::classify(true, Some(own), false, later()),
        JoinKind::Rejoin
    );
}

#[test]
fn only_a_rejoin_resets_modules() {
    assert!(JoinKind::Rejoin.resets_modules());
    assert!(!JoinKind::First.resets_modules());
    assert!(!JoinKind::Reconnect.resets_modules());
    assert!(!JoinKind::Additional.resets_modules());
}

#[sqlx::test(migrations = "../migrations")]
async fn a_second_bot_joining_is_additional(pool: PgPool) {
    GuildPresence::joined(&pool, SHARD_0_GUILD, ZAYDEN, joined()).await.unwrap();
    mark_seeded(&pool, SHARD_0_GUILD).await;

    let kind =
        GuildPresence::joined(&pool, SHARD_0_GUILD, VIKTOR, later()).await.unwrap();

    assert_eq!(kind, JoinKind::Additional);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_bot_absent_since_reconcile_joining_beside_another_is_additional(
    pool: PgPool,
) {
    GuildPresence::joined(&pool, SHARD_0_GUILD, ZAYDEN, joined()).await.unwrap();
    mark_seeded(&pool, SHARD_0_GUILD).await;
    GuildPresence::reconcile(&pool, VIKTOR, Shard { id: 0, total: 1 }, &[])
        .await
        .unwrap();

    let kind =
        GuildPresence::joined(&pool, SHARD_0_GUILD, VIKTOR, later()).await.unwrap();

    assert_eq!(kind, JoinKind::Additional);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_gateway_reconnect_is_a_reconnect(pool: PgPool) {
    GuildPresence::joined(&pool, SHARD_0_GUILD, ZAYDEN, joined()).await.unwrap();
    mark_seeded(&pool, SHARD_0_GUILD).await;

    let kind =
        GuildPresence::joined(&pool, SHARD_0_GUILD, ZAYDEN, joined()).await.unwrap();

    assert_eq!(kind, JoinKind::Reconnect);
}

#[sqlx::test(migrations = "../migrations")]
async fn re_adding_after_being_removed_is_a_rejoin(pool: PgPool) {
    GuildPresence::joined(&pool, SHARD_0_GUILD, ZAYDEN, joined()).await.unwrap();
    mark_seeded(&pool, SHARD_0_GUILD).await;
    GuildPresence::left(&pool, SHARD_0_GUILD, ZAYDEN).await.unwrap();

    let kind =
        GuildPresence::joined(&pool, SHARD_0_GUILD, ZAYDEN, later()).await.unwrap();

    assert_eq!(kind, JoinKind::Rejoin);
}
