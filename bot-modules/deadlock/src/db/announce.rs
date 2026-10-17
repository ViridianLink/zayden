use serenity::all::ChannelId;
use sqlx::PgPool;
use zayden_core::as_u64;

#[derive(Debug, Clone)]
pub struct PatchAnnounceRow {
    pub guild_id: i64,
    pub channel_id: i64,
    pub message: Option<String>,
    pub ping_role_ids: Vec<i64>,
}

impl PatchAnnounceRow {
    #[must_use]
    pub const fn channel(&self) -> ChannelId {
        ChannelId::new(as_u64(self.channel_id))
    }

    pub async fn select(pool: &PgPool, guild_id: i64) -> sqlx::Result<Option<Self>> {
        let row = sqlx::query_as!(
            Self,
            r#"
            SELECT guild_id, channel_id, message, ping_role_ids
            FROM deadlock_patch_announce
            WHERE guild_id = $1
            "#,
            guild_id
        )
        .fetch_optional(pool)
        .await?;

        Ok(row)
    }

    pub async fn upsert(
        pool: &PgPool,
        guild_id: i64,
        channel_id: i64,
        message: Option<&str>,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
            guild_id
        )
        .execute(pool)
        .await?;

        sqlx::query!(
            r#"
            INSERT INTO deadlock_patch_announce (guild_id, channel_id, message)
            VALUES ($1, $2, $3)
            ON CONFLICT (guild_id) DO UPDATE
            SET channel_id = EXCLUDED.channel_id,
                message = EXCLUDED.message,
                updated_at = now()
            "#,
            guild_id,
            channel_id,
            message
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn add_ping_role(
        pool: &PgPool,
        guild_id: i64,
        role_id: i64,
        max_roles: i32,
    ) -> sqlx::Result<bool> {
        let updated = sqlx::query!(
            r#"
            UPDATE deadlock_patch_announce
            SET ping_role_ids = array_append(ping_role_ids, $2),
                updated_at = now()
            WHERE guild_id = $1
              AND NOT ($2 = ANY (ping_role_ids))
              AND cardinality(ping_role_ids) < $3
            "#,
            guild_id,
            role_id,
            max_roles
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(updated > 0)
    }

    pub async fn remove_ping_role(
        pool: &PgPool,
        guild_id: i64,
        role_id: i64,
    ) -> sqlx::Result<bool> {
        let updated = sqlx::query!(
            r#"
            UPDATE deadlock_patch_announce
            SET ping_role_ids = array_remove(ping_role_ids, $2),
                updated_at = now()
            WHERE guild_id = $1 AND $2 = ANY (ping_role_ids)
            "#,
            guild_id,
            role_id
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(updated > 0)
    }

    pub async fn delete(pool: &PgPool, guild_id: i64) -> sqlx::Result<bool> {
        let deleted = sqlx::query!(
            "DELETE FROM deadlock_patch_announce WHERE guild_id = $1",
            guild_id
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(deleted > 0)
    }
}

pub async fn targets(
    pool: &PgPool,
    application_id: i64,
) -> sqlx::Result<Vec<PatchAnnounceRow>> {
    let rows = sqlx::query_as!(
        PatchAnnounceRow,
        r#"
        SELECT a.guild_id, a.channel_id, a.message, a.ping_role_ids
        FROM deadlock_patch_announce a
        WHERE NOT EXISTS (
                SELECT 1 FROM guild_modules m
                WHERE m.guild_id = a.guild_id
                  AND m.module = 'deadlock'
                  AND NOT m.enabled
            )
          AND EXISTS (
                SELECT 1 FROM guild_presence p
                WHERE p.guild_id = a.guild_id
                  AND p.application_id = $1
                  AND p.left_at IS NULL
            )
          AND serving_application(a.guild_id) IS NULL
        "#,
        application_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows)
}
