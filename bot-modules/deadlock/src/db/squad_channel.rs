use sqlx::PgPool;

#[derive(Debug, Clone, Copy)]
pub struct SquadChannel {
    pub guild_id: i64,
    pub channel_id: i64,
    pub leaderboard: bool,
    pub leaderboard_message_id: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
pub struct SquadTarget {
    pub guild_id: i64,
    pub channel_id: i64,
}

pub async fn select(
    pool: &PgPool,
    guild_id: i64,
) -> sqlx::Result<Option<SquadChannel>> {
    sqlx::query_as!(
        SquadChannel,
        r#"
        SELECT guild_id, channel_id, leaderboard, leaderboard_message_id
        FROM deadlock_squad_channel
        WHERE guild_id = $1
        "#,
        guild_id
    )
    .fetch_optional(pool)
    .await
}

pub async fn upsert(
    pool: &PgPool,
    guild_id: i64,
    channel_id: i64,
    leaderboard: bool,
) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
        guild_id
    )
    .execute(pool)
    .await?;

    sqlx::query!(
        r#"
        INSERT INTO deadlock_squad_channel (guild_id, channel_id, leaderboard)
        VALUES ($1, $2, $3)
        ON CONFLICT (guild_id) DO UPDATE
        SET leaderboard_message_id = CASE
                WHEN deadlock_squad_channel.channel_id <> EXCLUDED.channel_id
                THEN NULL
                ELSE deadlock_squad_channel.leaderboard_message_id
            END,
            channel_id = EXCLUDED.channel_id,
            leaderboard = EXCLUDED.leaderboard,
            updated_at = now()
        "#,
        guild_id,
        channel_id,
        leaderboard
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn delete(pool: &PgPool, guild_id: i64) -> sqlx::Result<bool> {
    let deleted = sqlx::query!(
        "DELETE FROM deadlock_squad_channel WHERE guild_id = $1",
        guild_id
    )
    .execute(pool)
    .await?
    .rows_affected();

    Ok(deleted > 0)
}

pub async fn set_leaderboard_message(
    pool: &PgPool,
    guild_id: i64,
    message_id: i64,
) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE deadlock_squad_channel SET leaderboard_message_id = $2 WHERE guild_id = $1",
        guild_id,
        message_id
    )
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn targets_for(
    pool: &PgPool,
    discord_id: i64,
    application_id: i64,
) -> sqlx::Result<Vec<SquadTarget>> {
    sqlx::query_as!(
        SquadTarget,
        r#"
        SELECT c.guild_id, c.channel_id
        FROM deadlock_squad_channel c
        JOIN deadlock_guild_members m
          ON m.guild_id = c.guild_id
         AND m.discord_id = $1
         AND m.squad
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

pub async fn leaderboard_guilds(
    pool: &PgPool,
    application_id: i64,
) -> sqlx::Result<Vec<SquadChannel>> {
    sqlx::query_as!(
        SquadChannel,
        r#"
        SELECT c.guild_id, c.channel_id, c.leaderboard, c.leaderboard_message_id
        FROM deadlock_squad_channel c
        WHERE c.leaderboard
          AND NOT EXISTS (
                SELECT 1 FROM guild_modules gm
                WHERE gm.guild_id = c.guild_id
                  AND gm.module = 'deadlock'
                  AND NOT gm.enabled
            )
          AND EXISTS (
                SELECT 1 FROM guild_presence p
                WHERE p.guild_id = c.guild_id
                  AND p.application_id = $1
                  AND p.left_at IS NULL
            )
          AND serving_application(c.guild_id) IS NULL
        ORDER BY c.guild_id
        "#,
        application_id
    )
    .fetch_all(pool)
    .await
}
