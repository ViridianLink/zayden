use jiff::Timestamp;
use jiff_sqlx::ToSqlx;
use sqlx::PgPool;

#[expect(
    trivial_casts,
    reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
)]
pub async fn insert(
    pool: &PgPool,
    account_id: u32,
    badge: i32,
    at: Timestamp,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO deadlock_rank_snapshots (account_id, captured_at, badge)
        VALUES ($1, $2, $3)
        ON CONFLICT DO NOTHING
        "#,
        i64::from(account_id),
        at.to_sqlx() as jiff_sqlx::Timestamp,
        badge
    )
    .execute(pool)
    .await?;

    Ok(())
}
