use jiff::Timestamp;
use jiff_sqlx::ToSqlx;
use sqlx::PgPool;

pub const RETENTION_DAYS: i32 = 30;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinKind {
    First,
    Reconnect,
    Additional,
    Rejoin,
}

#[derive(Debug, Clone, Copy)]
pub struct OwnPresence {
    pub joined_at: Option<Timestamp>,
    pub present: bool,
}

impl JoinKind {
    #[must_use]
    pub fn classify(
        seeded: bool,
        own: Option<OwnPresence>,
        others_present: bool,
        joined_at: Timestamp,
    ) -> Self {
        let same_membership = own.is_some_and(|own| {
            own.present && own.joined_at.is_none_or(|at| at == joined_at)
        });

        if !seeded {
            Self::First
        } else if same_membership {
            Self::Reconnect
        } else if others_present {
            Self::Additional
        } else {
            Self::Rejoin
        }
    }

    #[must_use]
    pub const fn resets_modules(self) -> bool {
        matches!(self, Self::Rejoin)
    }
}

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
        joined_at: Timestamp,
    ) -> sqlx::Result<JoinKind> {
        let mut tx = pool.begin().await?;

        sqlx::query!(
            "INSERT INTO guilds (id) VALUES ($1) ON CONFLICT (id) DO NOTHING",
            guild_id
        )
        .execute(&mut *tx)
        .await?;

        let seeded = sqlx::query_scalar!(
            r#"SELECT bot_joined_at IS NOT NULL AS "seeded!" FROM guilds WHERE id = $1 FOR UPDATE"#,
            guild_id
        )
        .fetch_one(&mut *tx)
        .await?;

        let own = sqlx::query!(
            r#"SELECT joined_at AS "joined_at: jiff_sqlx::Timestamp",
                      left_at IS NULL AS "present!"
               FROM guild_presence
               WHERE guild_id = $1 AND application_id = $2"#,
            guild_id,
            application_id
        )
        .fetch_optional(&mut *tx)
        .await?
        .map(|row| OwnPresence {
            joined_at: row.joined_at.map(jiff_sqlx::Timestamp::to_jiff),
            present: row.present,
        });

        let others_present = sqlx::query_scalar!(
            r#"SELECT EXISTS (
                   SELECT 1 FROM guild_presence
                   WHERE guild_id = $1 AND application_id <> $2 AND left_at IS NULL
               ) AS "present!""#,
            guild_id,
            application_id
        )
        .fetch_one(&mut *tx)
        .await?;

        #[expect(
            trivial_casts,
            reason = "not a cast: `as T` is sqlx's bind-param type-override syntax, required because TIMESTAMPTZ has no built-in jiff mapping"
        )]
        sqlx::query!(
            "INSERT INTO guild_presence (guild_id, application_id, joined_at)
             VALUES ($1, $2, $3)
             ON CONFLICT (guild_id, application_id)
             DO UPDATE SET left_at = NULL, joined_at = EXCLUDED.joined_at",
            guild_id,
            application_id,
            joined_at.to_sqlx() as jiff_sqlx::Timestamp,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(JoinKind::classify(seeded, own, others_present, joined_at))
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

    pub async fn is_present(
        pool: &PgPool,
        guild_id: i64,
        application_id: i64,
    ) -> sqlx::Result<bool> {
        sqlx::query_scalar!(
            r#"SELECT EXISTS (
                   SELECT 1 FROM guild_presence
                   WHERE guild_id = $1 AND application_id = $2 AND left_at IS NULL
               ) AS "present!""#,
            guild_id,
            application_id
        )
        .fetch_one(pool)
        .await
    }

    pub async fn present_guilds(
        pool: &PgPool,
        application_id: i64,
    ) -> sqlx::Result<Vec<i64>> {
        sqlx::query_scalar!(
            "SELECT guild_id FROM guild_presence
             WHERE application_id = $1 AND left_at IS NULL
             ORDER BY guild_id",
            application_id
        )
        .fetch_all(pool)
        .await
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
