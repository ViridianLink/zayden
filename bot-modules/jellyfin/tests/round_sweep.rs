//! Round retention for `/jellyfin guess` and `/watch trivia`.
//!
//! Rounds are write-once scratch state: `claim` refuses anything past
//! `expires_at`, and scores are settled into `jellyfin_game_scores`, so a round
//! is dead five minutes after it opens. Without the sweep the table grows for
//! the life of the deployment.
//!
//! These need a live Postgres — each `#[sqlx::test]` creates and drops its own
//! migrated database, so `DATABASE_URL` must point at a throwaway server.

use jellyfin::games::score::POINTS_CORRECT;
use jellyfin::games::{RoundRow, ScoreRow};
use jiff::{Span, Timestamp};
use serenity::all::{GenericChannelId, GuildId, UserId};
use sqlx::PgPool;

const GUILD: GuildId = GuildId::new(1_089_479_616_209_539_112);
const CHANNEL: GenericChannelId = GenericChannelId::new(1_089_479_616_209_539_113);
const STARTER: UserId = UserId::new(211_486_447_369_322_506);

async fn seed(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
        GUILD.get().cast_signed()
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        "INSERT INTO users (id, username) VALUES ($1, $2)",
        STARTER.get().cast_signed(),
        "starter"
    )
    .execute(pool)
    .await
    .map(|_| ())
}

/// `NewRound::open` always dates a round five minutes out, so the fixture
/// writes `expires_at` directly to reach either side of the retention window.
#[expect(
    trivial_casts,
    reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
)]
async fn round_expiring(pool: &PgPool, offset: Span) -> sqlx::Result<i64> {
    let expires_at = Timestamp::now() + offset;

    sqlx::query_scalar!(
        r#"
        INSERT INTO jellyfin_game_rounds
            (guild_id, channel_id, started_by, game, kind, answer, expires_at)
        VALUES ($1, $2, $3, 'guess', 'visual', 'Arrival', $4)
        RETURNING id
        "#,
        GUILD.get().cast_signed(),
        CHANNEL.get().cast_signed(),
        STARTER.get().cast_signed(),
        jiff_sqlx::Timestamp::from(expires_at) as jiff_sqlx::Timestamp,
    )
    .fetch_one(pool)
    .await
}

async fn survives(pool: &PgPool, id: i64) -> sqlx::Result<bool> {
    RoundRow::get(pool, id).await.map(|round| round.is_some())
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_round_that_expired_over_a_day_ago_is_swept(pool: PgPool) {
    seed(&pool).await.expect("seeding failed");
    let stale = round_expiring(&pool, Span::new().hours(-25))
        .await
        .expect("inserting the stale round failed");

    let removed = RoundRow::sweep_finished(&pool).await.expect("sweep failed");

    let kept = survives(&pool, stale).await.expect("round lookup failed");

    assert_eq!(removed, 1, "the sweep reported the wrong count");
    assert!(!kept, "the stale round outlived the sweep");
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_solved_round_is_swept_without_touching_its_score(pool: PgPool) {
    seed(&pool).await.expect("seeding failed");
    let stale = round_expiring(&pool, Span::new().hours(-25))
        .await
        .expect("inserting the stale round failed");

    RoundRow::claim(&pool, stale, STARTER, "starter").await.expect("claim failed");
    ScoreRow::record(&pool, GUILD, STARTER, "starter", "guess", true)
        .await
        .expect("recording the score failed");

    RoundRow::sweep_finished(&pool).await.expect("sweep failed");

    let kept = survives(&pool, stale).await.expect("round lookup failed");

    assert!(!kept, "the solved round outlived the sweep");

    let points = sqlx::query_scalar!(
        "SELECT points FROM jellyfin_game_scores WHERE user_id = $1",
        STARTER.get().cast_signed()
    )
    .fetch_optional(&pool)
    .await
    .expect("score lookup failed");

    assert_eq!(
        points,
        Some(POINTS_CORRECT),
        "the sweep took the leaderboard with it"
    );
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_live_round_survives(pool: PgPool) {
    seed(&pool).await.expect("seeding failed");
    let live = round_expiring(&pool, Span::new().minutes(5))
        .await
        .expect("inserting the live round failed");

    let removed = RoundRow::sweep_finished(&pool).await.expect("sweep failed");
    let kept = survives(&pool, live).await.expect("round lookup failed");

    assert_eq!(removed, 0, "the sweep took a round that is still playable");
    assert!(kept, "the live round was swept");
}

#[sqlx::test(migrations = "../../migrations")]
async fn a_round_inside_the_grace_window_survives(pool: PgPool) {
    seed(&pool).await.expect("seeding failed");
    let recent = round_expiring(&pool, Span::new().hours(-2))
        .await
        .expect("inserting the recent round failed");

    let removed = RoundRow::sweep_finished(&pool).await.expect("sweep failed");
    let kept = survives(&pool, recent).await.expect("round lookup failed");

    assert_eq!(removed, 0, "the grace window did not hold");
    assert!(kept, "the recent round was swept early");
}
