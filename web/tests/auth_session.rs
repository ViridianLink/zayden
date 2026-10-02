//! `lookup_session` is the only read of `web_sessions`: the miss fills the
//! cache and the hit skips Postgres. `guild_admin_for` decides guild access
//! from the cached guild list and `web_user_roles`.

use std::error::Error;
use std::sync::Arc;
use std::time::Duration;

use jiff::{SignedDuration, Timestamp};
use moka::future::Cache;
use sqlx::PgPool;
use twilight_model::guild::Permissions;
use twilight_model::id::Id;
use twilight_model::user::CurrentUserGuild;
use web::auth::{AuthError, GuildAccess, guild_admin_for, lookup_session};
use web::state::{SessionCache, SessionIdentity, UserGuildsCache};

type TestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// Seeded with both dashboard roles by the migrations.
const OPERATOR: i64 = 211_486_447_369_322_506;

async fn insert_session(
    pool: &PgPool,
    token: &str,
    user_id: i64,
    ttl: SignedDuration,
) -> TestResult<()> {
    let expires_at = jiff_sqlx::Timestamp::from(Timestamp::now().checked_add(ttl)?);

    #[expect(
        trivial_casts,
        reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
    )]
    sqlx::query!(
        "INSERT INTO web_sessions \
             (token, discord_user_id, discord_access_token, expires_at) \
         VALUES ($1, $2, $3, $4)",
        token,
        user_id,
        format!("{token}-access"),
        expires_at as jiff_sqlx::Timestamp
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn seed(pool: &PgPool) -> TestResult<()> {
    insert_session(pool, "live-token", 41, SignedDuration::from_hours(168)).await?;
    insert_session(pool, "expired-token", 42, SignedDuration::from_hours(-1))
        .await?;
    Ok(())
}

fn session_cache() -> SessionCache {
    Cache::builder().max_capacity(16).time_to_live(Duration::from_secs(60)).build()
}

#[sqlx::test(migrations = "../migrations")]
async fn a_miss_reads_the_row_and_records_it(pool: PgPool) {
    seed(&pool).await.unwrap();
    let cache = session_cache();

    let identity =
        lookup_session(Some(&cache), &pool, "live-token").await.unwrap().unwrap();

    assert_eq!(identity.user_id, 41);
    assert_eq!(identity.access_token, "live-token-access");

    let cached = cache.get("live-token").await.unwrap();
    assert_eq!(cached.user_id, 41);
    assert_eq!(cached.access_token, "live-token-access");
}

/// The token has no row: only the cache can answer.
#[sqlx::test(migrations = "../migrations")]
async fn a_hit_answers_without_the_database(pool: PgPool) {
    let cache = session_cache();
    cache
        .insert("cached-only-token".to_owned(), SessionIdentity {
            user_id: 77,
            access_token: "cached-only-access-token".to_owned(),
        })
        .await;

    let identity = lookup_session(Some(&cache), &pool, "cached-only-token")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(identity.user_id, 77);
    assert_eq!(identity.access_token, "cached-only-access-token");
}

#[sqlx::test(migrations = "../migrations")]
async fn an_unknown_token_is_not_a_session(pool: PgPool) {
    seed(&pool).await.unwrap();
    let cache = session_cache();

    let identity =
        lookup_session(Some(&cache), &pool, "no-such-token").await.unwrap();

    assert!(identity.is_none());
    assert!(cache.get("no-such-token").await.is_none());
}

#[sqlx::test(migrations = "../migrations")]
async fn an_expired_session_is_not_revived_by_the_cache(pool: PgPool) {
    seed(&pool).await.unwrap();
    let cache = session_cache();

    let identity =
        lookup_session(Some(&cache), &pool, "expired-token").await.unwrap();

    assert!(identity.is_none());
    assert!(cache.get("expired-token").await.is_none());
}

#[sqlx::test(migrations = "../migrations")]
async fn the_lookup_works_with_no_cache_at_all(pool: PgPool) {
    seed(&pool).await.unwrap();

    let identity = lookup_session(None, &pool, "live-token").await.unwrap().unwrap();

    assert_eq!(identity.user_id, 41);
}

fn guild(id: u64, permissions: Permissions) -> CurrentUserGuild {
    CurrentUserGuild {
        id: Id::new(id),
        name: format!("Guild {id}"),
        icon: None,
        owner: false,
        permissions,
        features: Vec::new(),
    }
}

/// The access token is unusable, so every decision below comes from the
/// cached guild list rather than Discord.
async fn guilds_cache(user_id: i64) -> UserGuildsCache {
    let cache: UserGuildsCache = Cache::builder()
        .max_capacity(16)
        .time_to_live(Duration::from_secs(60))
        .build();
    cache
        .insert(
            user_id,
            Arc::from([
                guild(7, Permissions::MANAGE_GUILD),
                guild(8, Permissions::SEND_MESSAGES),
            ]),
        )
        .await;
    cache
}

fn identity(user_id: i64) -> SessionIdentity {
    SessionIdentity { user_id, access_token: "not-a-real-access-token".to_owned() }
}

#[sqlx::test(migrations = "../migrations")]
async fn a_member_who_manages_the_guild_gets_member_access(pool: PgPool) {
    let cache = guilds_cache(41).await;

    let ctx = guild_admin_for(&pool, &identity(41), "7", None, Some(&cache))
        .await
        .unwrap();

    assert_eq!(ctx.guild_id, 7);
    assert_eq!(ctx.access, GuildAccess::Member);
    assert_eq!(ctx.access_token, "not-a-real-access-token");
}

/// The context carries the user's bearer token; `Debug` output, which ends up
/// in logs and panic messages, must not.
#[sqlx::test(migrations = "../migrations")]
async fn the_guild_context_debug_output_hides_the_token(pool: PgPool) {
    let cache = guilds_cache(41).await;

    let ctx = guild_admin_for(&pool, &identity(41), "7", None, Some(&cache))
        .await
        .unwrap();

    assert_eq!(
        format!("{ctx:?}"),
        "GuildAdminContext { guild_id: 7, access: Member, .. }"
    );
}

#[sqlx::test(migrations = "../migrations")]
async fn a_member_without_manage_rights_is_forbidden(pool: PgPool) {
    let cache = guilds_cache(41).await;

    let error = guild_admin_for(&pool, &identity(41), "8", None, Some(&cache))
        .await
        .unwrap_err();

    assert_eq!(error, AuthError::Forbidden);
}

#[sqlx::test(migrations = "../migrations")]
async fn a_non_numeric_guild_id_is_rejected(pool: PgPool) {
    let cache = guilds_cache(41).await;

    let error = guild_admin_for(&pool, &identity(41), "abc", None, Some(&cache))
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "invalid guild id");
}

/// With no bot client the bot cannot be in the guild, which is the refusal an
/// operator gets for a guild Zayden has left.
#[sqlx::test(migrations = "../migrations")]
async fn an_operator_needs_the_bot_in_the_guild(pool: PgPool) {
    let cache = guilds_cache(OPERATOR).await;

    let error = guild_admin_for(&pool, &identity(OPERATOR), "9", None, Some(&cache))
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "Zayden isn't in that server");
}

#[sqlx::test(migrations = "../migrations")]
async fn an_operator_who_manages_the_guild_is_a_member(pool: PgPool) {
    let cache = guilds_cache(OPERATOR).await;

    let ctx = guild_admin_for(&pool, &identity(OPERATOR), "7", None, Some(&cache))
        .await
        .unwrap();

    assert_eq!(ctx.access, GuildAccess::Member);
}
