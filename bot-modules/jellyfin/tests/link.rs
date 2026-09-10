//! Regression tests for `JellyfinLinkRow::insert` and the `users` row it must
//! create first.
//!
//! `users.username` is `VARCHAR(255) NOT NULL` with no default
//! ([`0001_v1_init.up.sql`]). `insert` used to seed the parent row with
//! `INSERT INTO users (id) VALUES ($1) ON CONFLICT (id) DO NOTHING`, which
//! reads as safe but is not. Postgres validates `NOT NULL` while forming the
//! candidate tuple, which happens *before* `ON CONFLICT` arbitration, so the
//! `DO NOTHING` never got a chance to swallow it: every link hit `23502`,
//! whether or not the `users` row already existed. Quick connect surfaced that
//! as a bare `JellyfinError::Sqlx` after the Jellyfin handshake had already
//! succeeded, stranding the account half-linked.
//!
//! These need a live Postgres — each `#[sqlx::test]` creates and drops its own
//! migrated database, so `DATABASE_URL` must point at a throwaway server.
//!
//! [`0001_v1_init.up.sql`]: ../../../migrations/0001_v1_init.up.sql

use jellyfin::JellyfinLinkRow;
use serenity::all::UserId;
use sqlx::PgPool;

/// The id from the production failure this suite was written for.
const FIRST_TIME_LINKER: UserId = UserId::new(211_486_447_369_322_506);

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

/// Catches reverting the parent-row seed to `INSERT INTO users (id) VALUES ($1)`,
/// which fails with `23502` for anyone who has no `users` row yet.
#[sqlx::test(migrations = "../../migrations")]
async fn links_a_user_who_has_no_users_row_yet(pool: PgPool) {
    assert_eq!(
        username_of(&pool, FIRST_TIME_LINKER).await.expect("users lookup failed"),
        None,
        "the fixture must not pre-seed the actor, or the test proves nothing"
    );

    JellyfinLinkRow::insert(
        &pool,
        FIRST_TIME_LINKER,
        "discord-name",
        "jf-user-id",
        "jellyfin-name",
    )
    .await
    .expect(
        "a first-time linker must not hit the users.username NOT NULL constraint",
    );

    assert_eq!(
        username_of(&pool, FIRST_TIME_LINKER)
            .await
            .expect("users lookup failed")
            .as_deref(),
        Some("discord-name"),
        "the parent row carries the Discord username, not a placeholder"
    );

    let row = JellyfinLinkRow::get(&pool, FIRST_TIME_LINKER)
        .await
        .expect("link lookup failed")
        .expect("the link row was written");
    assert_eq!(row.jellyfin_user_id, "jf-user-id");
    assert_eq!(row.jellyfin_username, "jellyfin-name");
}

/// The parent seed is `ON CONFLICT DO NOTHING`, so a `users` row another module
/// already owns keeps its username — linking Jellyfin must not rename anyone.
#[sqlx::test(migrations = "../../migrations")]
async fn keeps_the_username_an_existing_users_row_already_has(pool: PgPool) {
    sqlx::query!(
        "INSERT INTO users (id, username) VALUES ($1, $2)",
        FIRST_TIME_LINKER.get().cast_signed(),
        "name-set-by-another-module"
    )
    .execute(&pool)
    .await
    .expect("seeding the users row failed");

    JellyfinLinkRow::insert(
        &pool,
        FIRST_TIME_LINKER,
        "stale-name",
        "jf-user-id",
        "jellyfin-name",
    )
    .await
    .expect("linking over an existing users row must succeed");

    assert_eq!(
        username_of(&pool, FIRST_TIME_LINKER)
            .await
            .expect("users lookup failed")
            .as_deref(),
        Some("name-set-by-another-module"),
        "ON CONFLICT DO NOTHING must leave the existing username alone"
    );
}
