use jiff::Timestamp;
use jiff_sqlx::ToSqlx;
use sqlx::PgPool;

#[derive(Debug, Clone, Copy)]
pub struct SeenMatch {
    pub account_id: u32,
    pub match_id: u64,
    pub hero_id: u32,
    pub won: bool,
    pub start_time: Timestamp,
}

#[expect(
    trivial_casts,
    reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
)]
pub async fn insert_if_new(pool: &PgPool, seen: &SeenMatch) -> sqlx::Result<bool> {
    let hero_id = i32::try_from(seen.hero_id)
        .map_err(|source| sqlx::Error::Encode(Box::new(source)))?;

    let inserted = sqlx::query_scalar!(
        r#"
        INSERT INTO deadlock_matches_seen (account_id, match_id, hero_id, won, start_time)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT DO NOTHING
        RETURNING match_id
        "#,
        i64::from(seen.account_id),
        seen.match_id.cast_signed(),
        hero_id,
        seen.won,
        seen.start_time.to_sqlx() as jiff_sqlx::Timestamp
    )
    .fetch_optional(pool)
    .await?;

    Ok(inserted.is_some())
}
