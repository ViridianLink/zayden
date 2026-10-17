use sqlx::PgPool;

pub async fn claim(
    pool: &PgPool,
    guild_id: i64,
    match_id: u64,
) -> sqlx::Result<bool> {
    let claimed = sqlx::query_scalar!(
        r#"
        INSERT INTO deadlock_guild_posts (guild_id, match_id)
        VALUES ($1, $2)
        ON CONFLICT DO NOTHING
        RETURNING match_id
        "#,
        guild_id,
        match_id.cast_signed()
    )
    .fetch_optional(pool)
    .await?;

    Ok(claimed.is_some())
}
