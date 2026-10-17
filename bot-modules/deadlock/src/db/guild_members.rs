use sqlx::PgPool;

pub async fn share_summaries(
    pool: &PgPool,
    guild_id: i64,
    discord_id: i64,
) -> sqlx::Result<bool> {
    let enabled = sqlx::query_scalar!(
        r#"
        SELECT share_summaries
        FROM deadlock_guild_members
        WHERE guild_id = $1 AND discord_id = $2
        "#,
        guild_id,
        discord_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(enabled.unwrap_or(false))
}

pub async fn set_share_summaries(
    pool: &PgPool,
    guild_id: i64,
    discord_id: i64,
    enabled: bool,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
        guild_id
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO deadlock_guild_members (guild_id, discord_id, share_summaries)
        VALUES ($1, $2, $3)
        ON CONFLICT (guild_id, discord_id) DO UPDATE
        SET share_summaries = EXCLUDED.share_summaries,
            updated_at = now()
        "#,
        guild_id,
        discord_id,
        enabled
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn shared_guilds(
    pool: &PgPool,
    discord_id: i64,
) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar!(
        r#"
        SELECT guild_id
        FROM deadlock_guild_members
        WHERE discord_id = $1 AND share_summaries
        ORDER BY guild_id
        "#,
        discord_id
    )
    .fetch_all(pool)
    .await
}
