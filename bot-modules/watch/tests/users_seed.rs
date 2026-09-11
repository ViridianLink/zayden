//! Regression tests for the `users` parent row that `watch`'s four write paths
//! must seed before they can reference it.
//!
//! `users.username` is `VARCHAR(255) NOT NULL` with no default
//! ([`0001_v1_init.up.sql`]). These paths used to seed the parent row with
//! `INSERT INTO users (id) VALUES ($1) ON CONFLICT (id) DO NOTHING`, which
//! reads as safe but is not. Postgres validates `NOT NULL` while forming the
//! candidate tuple, which happens *before* `ON CONFLICT` arbitration, so
//! `DO NOTHING` never got a chance to swallow it: anyone without a `users` row
//! hit `23502` instead of playing a round or joining a party.
//!
//! Each test therefore uses an actor with no pre-existing `users` row — the
//! exact case that failed — and asserts the row lands carrying the real
//! Discord username rather than a placeholder.
//!
//! These need a live Postgres — each `#[sqlx::test]` creates and drops its own
//! migrated database, so `DATABASE_URL` must point at a throwaway server.
//!
//! [`0001_v1_init.up.sql`]: ../../../migrations/0001_v1_init.up.sql

use jellyfin::LibraryItemRow;
use jiff::{Span, Timestamp};
use serenity::all::{GenericChannelId, GuildId, UserId};
use sqlx::PgPool;
use watch::games::{NewRound, RoundRow, ScoreRow};
use watch::party::{PartyGuestRow, PartyRow};

const GUILD: GuildId = GuildId::new(1_089_479_616_209_539_112);
const CHANNEL: GenericChannelId = GenericChannelId::new(1_089_479_616_209_539_113);

/// The actor under test in each case: never seeded, so the seed must do it.
const NEWCOMER: UserId = UserId::new(211_486_447_369_322_506);
/// A second party who does have a row, used to set up rounds and parties.
const REGULAR: UserId = UserId::new(211_486_447_369_322_507);

async fn username_of(
    pool: &PgPool,
    user_id: UserId,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar!(
        "SELECT username FROM users WHERE id = $1",
        user_id.get().cast_signed()
    )
    .fetch_optional(pool)
    .await
}

/// `NewRound::open` and `PartyRow::insert` reference `guilds` without seeding
/// it, so the fixture stands in for the guild the bot has already joined.
async fn seed_guild(pool: &PgPool) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
        GUILD.get().cast_signed()
    )
    .execute(pool)
    .await
    .map(|_| ())
}

async fn seed_user(
    pool: &PgPool,
    user_id: UserId,
    username: &str,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO users (id, username) VALUES ($1, $2)",
        user_id.get().cast_signed(),
        username
    )
    .execute(pool)
    .await
    .map(|_| ())
}

const fn round_started_by(
    started_by: UserId,
    started_by_name: &str,
) -> NewRound<'_> {
    NewRound {
        guild_id: GUILD,
        channel_id: CHANNEL,
        started_by,
        started_by_name,
        game: "guess",
        kind: "movie",
        answer: "Arrival",
        choices: None,
        reveal_image_url: None,
    }
}

/// Catches reverting the seed in `NewRound::open` to `INSERT INTO users (id)`,
/// which fails with `23502` for anyone starting their first round.
#[sqlx::test(migrations = "../../migrations")]
async fn opens_a_round_for_a_starter_who_has_no_users_row_yet(pool: PgPool) {
    seed_guild(&pool).await.expect("seeding the guilds row failed");
    assert_eq!(
        username_of(&pool, NEWCOMER).await.expect("users lookup failed"),
        None,
        "the fixture must not pre-seed the actor, or the test proves nothing"
    );

    let id = round_started_by(NEWCOMER, "starter-name").open(&pool).await.expect(
        "a first-time starter must not hit the username NOT NULL constraint",
    );

    assert_eq!(
        username_of(&pool, NEWCOMER).await.expect("users lookup failed").as_deref(),
        Some("starter-name"),
        "the parent row carries the Discord username, not a placeholder"
    );

    let round = RoundRow::get(&pool, id)
        .await
        .expect("round lookup failed")
        .expect("the round row was written");
    assert_eq!(round.answer, "Arrival");
}

/// Catches reverting the seed in `RoundRow::claim`: the solver is usually not
/// the starter, so the first person to answer may have no `users` row at all.
#[sqlx::test(migrations = "../../migrations")]
async fn claims_a_round_for_a_solver_who_has_no_users_row_yet(pool: PgPool) {
    seed_guild(&pool).await.expect("seeding the guilds row failed");
    seed_user(&pool, REGULAR, "the-starter")
        .await
        .expect("seeding the users row failed");

    let id = round_started_by(REGULAR, "the-starter")
        .open(&pool)
        .await
        .expect("opening the round failed");

    assert_eq!(
        username_of(&pool, NEWCOMER).await.expect("users lookup failed"),
        None,
        "the fixture must not pre-seed the actor, or the test proves nothing"
    );

    let answer = RoundRow::claim(&pool, id, NEWCOMER, "solver-name")
        .await
        .expect("a first-time solver must not hit the username NOT NULL constraint")
        .expect("an open, unexpired round is claimable");
    assert_eq!(answer, "Arrival");

    assert_eq!(
        username_of(&pool, NEWCOMER).await.expect("users lookup failed").as_deref(),
        Some("solver-name"),
        "the parent row carries the Discord username, not a placeholder"
    );
}

/// Catches reverting the seed in `ScoreRow::record`, which runs on every
/// settled answer — including the player's very first.
#[sqlx::test(migrations = "../../migrations")]
async fn records_a_score_for_a_player_who_has_no_users_row_yet(pool: PgPool) {
    assert_eq!(
        username_of(&pool, NEWCOMER).await.expect("users lookup failed"),
        None,
        "the fixture must not pre-seed the actor, or the test proves nothing"
    );

    ScoreRow::record(&pool, GUILD, NEWCOMER, "player-name", "guess", true)
        .await
        .expect("a first-time player must not hit the username NOT NULL constraint");

    assert_eq!(
        username_of(&pool, NEWCOMER).await.expect("users lookup failed").as_deref(),
        Some("player-name"),
        "the parent row carries the Discord username, not a placeholder"
    );

    let board = ScoreRow::leaderboard(&pool, GUILD, Some("guess"), 10)
        .await
        .expect("leaderboard lookup failed");
    let row = board.first().expect("the score row was written");
    assert_eq!(row.user(), NEWCOMER);
    assert_eq!(row.correct, 1);
    assert_eq!(row.played, 1);
}

/// Catches reverting the seed in `PartyGuestRow::join`: a guest joining a
/// party is the likeliest actor in the whole module to be new to the bot.
#[sqlx::test(migrations = "../../migrations")]
async fn joins_a_party_for_a_guest_who_has_no_users_row_yet(pool: PgPool) {
    seed_guild(&pool).await.expect("seeding the guilds row failed");
    seed_user(&pool, REGULAR, "the-host")
        .await
        .expect("seeding the users row failed");

    let starts_at = Timestamp::now() + Span::new().hours(1);
    let ends_at = starts_at + Span::new().hours(2);
    let party_id = PartyRow::insert(
        &pool,
        GUILD,
        CHANNEL,
        REGULAR,
        &library_item(),
        "/media/movies",
        true,
        starts_at,
        ends_at,
        ends_at + Span::new().hours(1),
    )
    .await
    .expect("creating the party failed");

    assert_eq!(
        username_of(&pool, NEWCOMER).await.expect("users lookup failed"),
        None,
        "the fixture must not pre-seed the actor, or the test proves nothing"
    );

    PartyGuestRow::join(&pool, party_id, NEWCOMER, "guest-name", "jellyfin-name")
        .await
        .expect("a first-time guest must not hit the username NOT NULL constraint");

    assert_eq!(
        username_of(&pool, NEWCOMER).await.expect("users lookup failed").as_deref(),
        Some("guest-name"),
        "the parent row carries the Discord username, not a placeholder"
    );

    let guests = PartyGuestRow::for_party(&pool, party_id)
        .await
        .expect("guest lookup failed");
    let guest = guests.first().expect("the guest row was written");
    assert_eq!(guest.user(), NEWCOMER);
    assert_eq!(
        guest.jellyfin_username, "jellyfin-name",
        "the guest row keeps the Jellyfin name, distinct from the Discord one"
    );
}

/// The seed is `ON CONFLICT DO NOTHING`, so a `users` row another module
/// already owns keeps its username — playing a round must not rename anyone.
#[sqlx::test(migrations = "../../migrations")]
async fn keeps_the_username_an_existing_users_row_already_has(pool: PgPool) {
    seed_guild(&pool).await.expect("seeding the guilds row failed");
    seed_user(&pool, NEWCOMER, "name-set-by-another-module")
        .await
        .expect("seeding the users row failed");

    let id = round_started_by(NEWCOMER, "stale-name")
        .open(&pool)
        .await
        .expect("opening the round failed");
    RoundRow::claim(&pool, id, NEWCOMER, "stale-name")
        .await
        .expect("claiming the round failed")
        .expect("an open, unexpired round is claimable");
    ScoreRow::record(&pool, GUILD, NEWCOMER, "stale-name", "guess", true)
        .await
        .expect("recording the score failed");

    assert_eq!(
        username_of(&pool, NEWCOMER).await.expect("users lookup failed").as_deref(),
        Some("name-set-by-another-module"),
        "ON CONFLICT DO NOTHING must leave the existing username alone"
    );
}

fn library_item() -> LibraryItemRow {
    LibraryItemRow {
        item_id: "item-1".to_owned(),
        item_type: "Movie".to_owned(),
        name: "Arrival".to_owned(),
        sort_name: "Arrival".to_owned(),
        production_year: Some(2016),
        tmdb_id: None,
        imdb_id: None,
        tvdb_id: None,
        runtime_ticks: None,
        community_rating: None,
        genres: Vec::new(),
        parent_path: Some("/media/movies".to_owned()),
    }
}
