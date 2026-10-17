use jiff::Timestamp;
use jiff_sqlx::ToSqlx;
use sqlx::PgPool;

#[expect(
    trivial_casts,
    reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
)]
pub async fn prune_cache(pool: &PgPool, before: Timestamp) -> sqlx::Result<u64> {
    let result = sqlx::query!(
        "DELETE FROM deadlock_api_cache WHERE expires_at < $1",
        before.to_sqlx() as jiff_sqlx::Timestamp
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

#[expect(
    trivial_casts,
    reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
)]
pub async fn prune_rank_snapshots(
    pool: &PgPool,
    before: Timestamp,
) -> sqlx::Result<u64> {
    let result = sqlx::query!(
        "DELETE FROM deadlock_rank_snapshots WHERE captured_at < $1",
        before.to_sqlx() as jiff_sqlx::Timestamp
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

#[expect(
    trivial_casts,
    reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
)]
pub async fn prune_matches(pool: &PgPool, before: Timestamp) -> sqlx::Result<u64> {
    let result = sqlx::query!(
        "DELETE FROM deadlock_matches_seen WHERE start_time < $1",
        before.to_sqlx() as jiff_sqlx::Timestamp
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

#[expect(
    trivial_casts,
    reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
)]
pub async fn prune_guild_posts(
    pool: &PgPool,
    before: Timestamp,
) -> sqlx::Result<u64> {
    let result = sqlx::query!(
        "DELETE FROM deadlock_guild_posts WHERE posted_at < $1",
        before.to_sqlx() as jiff_sqlx::Timestamp
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

#[expect(
    trivial_casts,
    reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
)]
pub async fn prune_briefings(pool: &PgPool, before: Timestamp) -> sqlx::Result<u64> {
    let result = sqlx::query!(
        "DELETE FROM deadlock_briefings_sent WHERE sent_at < $1",
        before.to_sqlx() as jiff_sqlx::Timestamp
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}

pub async fn prune_asset_snapshots(pool: &PgPool, keep: i64) -> sqlx::Result<u64> {
    let result = sqlx::query!(
        r#"
        DELETE FROM deadlock_asset_snapshots
        WHERE client_version NOT IN (
                SELECT client_version FROM deadlock_asset_snapshots
                ORDER BY client_version DESC LIMIT $1)
          AND client_version NOT IN (SELECT client_version FROM deadlock_watch_state)
        "#,
        keep
    )
    .execute(pool)
    .await?;

    Ok(result.rows_affected())
}
