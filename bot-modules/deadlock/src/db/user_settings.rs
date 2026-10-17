use sqlx::PgPool;

pub async fn dm_summaries(pool: &PgPool, discord_id: i64) -> sqlx::Result<bool> {
    let enabled = sqlx::query_scalar!(
        "SELECT dm_summaries FROM deadlock_user_settings WHERE discord_id = $1",
        discord_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(enabled.unwrap_or(true))
}

pub async fn patch_briefing(pool: &PgPool, discord_id: i64) -> sqlx::Result<bool> {
    let enabled = sqlx::query_scalar!(
        "SELECT patch_briefing FROM deadlock_user_settings WHERE discord_id = $1",
        discord_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(enabled.unwrap_or(true))
}

pub async fn set_patch_briefing(
    pool: &PgPool,
    discord_id: i64,
    enabled: bool,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO deadlock_user_settings (discord_id, patch_briefing)
        VALUES ($1, $2)
        ON CONFLICT (discord_id) DO UPDATE
        SET patch_briefing = EXCLUDED.patch_briefing,
            updated_at = now()
        "#,
        discord_id,
        enabled
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn set_dm_summaries(
    pool: &PgPool,
    discord_id: i64,
    enabled: bool,
) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        INSERT INTO deadlock_user_settings (discord_id, dm_summaries)
        VALUES ($1, $2)
        ON CONFLICT (discord_id) DO UPDATE
        SET dm_summaries = EXCLUDED.dm_summaries,
            updated_at = now()
        "#,
        discord_id,
        enabled
    )
    .execute(pool)
    .await?;

    Ok(())
}
