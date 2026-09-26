use sqlx::PgPool;

pub const RETENTION_DAYS: i32 = 30;

#[derive(Debug, Clone, Copy)]
pub struct Shard {
    pub id: u16,
    pub total: u16,
}

pub struct GuildPresence;

impl GuildPresence {
    pub async fn joined(
        pool: &PgPool,
        guild_id: i64,
        application_id: i64,
    ) -> sqlx::Result<()> {
        let mut tx = pool.begin().await?;

        sqlx::query!(
            "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
            guild_id
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "INSERT INTO guild_presence (guild_id, application_id)
             VALUES ($1, $2)
             ON CONFLICT (guild_id, application_id) DO UPDATE SET left_at = NULL",
            guild_id,
            application_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await
    }

    pub async fn left(
        pool: &PgPool,
        guild_id: i64,
        application_id: i64,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO guild_presence (guild_id, application_id, left_at)
             SELECT $1, $2, now()
             WHERE EXISTS (SELECT 1 FROM guilds WHERE id = $1)
             ON CONFLICT (guild_id, application_id) DO UPDATE
             SET left_at = COALESCE(guild_presence.left_at, now())",
            guild_id,
            application_id
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn reconcile(
        pool: &PgPool,
        application_id: i64,
        shard: Shard,
        present: &[i64],
    ) -> sqlx::Result<u64> {
        let marked = sqlx::query!(
            "INSERT INTO guild_presence (guild_id, application_id, left_at)
             SELECT g.id, $1, now()
             FROM guilds g
             WHERE (g.id >> 22) % $2 = $3 AND g.id <> ALL($4)
             ON CONFLICT (guild_id, application_id) DO UPDATE
             SET left_at = COALESCE(guild_presence.left_at, now())
             WHERE guild_presence.left_at IS NULL",
            application_id,
            i64::from(shard.total.max(1)),
            i64::from(shard.id),
            present
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(marked)
    }

    pub async fn expired(
        pool: &PgPool,
        retention_days: i32,
    ) -> sqlx::Result<Vec<i64>> {
        sqlx::query_scalar!(
            "SELECT g.id
             FROM guilds g
             WHERE EXISTS (SELECT 1 FROM guild_presence p WHERE p.guild_id = g.id)
               AND NOT EXISTS (
                   SELECT 1 FROM guild_presence p
                   WHERE p.guild_id = g.id
                     AND (p.left_at IS NULL
                          OR p.left_at > now() - make_interval(days => $1))
               )
             ORDER BY g.id",
            retention_days
        )
        .fetch_all(pool)
        .await
    }

    pub async fn purge(
        pool: &PgPool,
        guild_id: i64,
        retention_days: i32,
    ) -> sqlx::Result<bool> {
        let deleted = sqlx::query!(
            "DELETE FROM guilds g
             WHERE g.id = $1
               AND EXISTS (SELECT 1 FROM guild_presence p WHERE p.guild_id = g.id)
               AND NOT EXISTS (
                   SELECT 1 FROM guild_presence p
                   WHERE p.guild_id = g.id
                     AND (p.left_at IS NULL
                          OR p.left_at > now() - make_interval(days => $2))
               )",
            guild_id,
            retention_days
        )
        .execute(pool)
        .await?
        .rows_affected();

        Ok(deleted > 0)
    }
}
