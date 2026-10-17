use sqlx::PgPool;

#[derive(Debug, Clone, Copy)]
pub struct SummaryTarget {
    pub guild_id: i64,
    pub channel_id: i64,
}

pub async fn select(pool: &PgPool, guild_id: i64) -> sqlx::Result<Option<i64>> {
    sqlx::query_scalar!(
        "SELECT channel_id FROM deadlock_summary_channel WHERE guild_id = $1",
        guild_id
    )
    .fetch_optional(pool)
    .await
}

pub async fn upsert(
    pool: &PgPool,
    guild_id: i64,
    channel_id: i64,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
        guild_id
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO deadlock_summary_channel (guild_id, channel_id)
        VALUES ($1, $2)
        ON CONFLICT (guild_id) DO UPDATE
        SET channel_id = EXCLUDED.channel_id,
            updated_at = now()
        "#,
        guild_id,
        channel_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete(pool: &PgPool, guild_id: i64) -> sqlx::Result<bool> {
    let deleted = sqlx::query!(
        "DELETE FROM deadlock_summary_channel WHERE guild_id = $1",
        guild_id
    )
    .execute(pool)
    .await?
    .rows_affected();

    Ok(deleted > 0)
}

pub async fn targets_for(
    pool: &PgPool,
    discord_id: i64,
    application_id: i64,
) -> sqlx::Result<Vec<SummaryTarget>> {
    sqlx::query_as!(
        SummaryTarget,
        r#"
        SELECT c.guild_id, c.channel_id
        FROM deadlock_summary_channel c
        JOIN deadlock_guild_members m
          ON m.guild_id = c.guild_id
         AND m.discord_id = $1
         AND m.share_summaries
        WHERE NOT EXISTS (
                SELECT 1 FROM guild_modules gm
                WHERE gm.guild_id = c.guild_id
                  AND gm.module = 'deadlock'
                  AND NOT gm.enabled
            )
          AND EXISTS (
                SELECT 1 FROM guild_presence p
                WHERE p.guild_id = c.guild_id
                  AND p.application_id = $2
                  AND p.left_at IS NULL
            )
          AND serving_application(c.guild_id) IS NULL
        ORDER BY c.guild_id
        "#,
        discord_id,
        application_id
    )
    .fetch_all(pool)
    .await
}
