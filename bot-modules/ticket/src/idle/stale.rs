use serenity::all::{ChannelId, ForumTagId, GuildId, ThreadId};
use sqlx::PgPool;
use zayden_core::as_u64;

pub struct StaleTarget {
    pub thread_id: i64,
    pub guild_id: i64,
    pub support_channel_id: Option<i64>,
    pub stale_tag_id: Option<i64>,
}

impl StaleTarget {
    #[must_use]
    pub const fn thread(&self) -> ThreadId {
        ThreadId::new(as_u64(self.thread_id))
    }

    #[must_use]
    pub const fn guild(&self) -> GuildId {
        GuildId::new(as_u64(self.guild_id))
    }

    #[must_use]
    pub fn support_channel(&self) -> Option<ChannelId> {
        self.support_channel_id.map(|id| ChannelId::new(as_u64(id)))
    }

    #[must_use]
    pub fn stale_tag(&self) -> Option<ForumTagId> {
        self.stale_tag_id.map(|id| ForumTagId::new(as_u64(id)))
    }
}

pub async fn claim_due(pool: &PgPool, limit: i64) -> sqlx::Result<Vec<StaleTarget>> {
    sqlx::query_as!(
        StaleTarget,
        r#"
        WITH due AS (
            SELECT a.thread_id
            FROM support_thread_activity a
            JOIN support_settings s ON s.guild_id = a.guild_id
            WHERE s.stale_enabled
              AND s.stale_tag_id IS NOT NULL
              AND NOT a.paused
              AND NOT a.waiting_on_helper
              AND a.staled_at IS NULL
              AND a.since < now() - (s.stale_after_secs * interval '1 second')
            ORDER BY a.guild_id, a.since
            LIMIT $1
            FOR UPDATE OF a SKIP LOCKED
        )
        UPDATE support_thread_activity a
        SET staled_at = now()
        FROM due, support_settings s
        WHERE a.thread_id = due.thread_id
          AND s.guild_id = a.guild_id
        RETURNING
            a.thread_id,
            a.guild_id,
            s.support_channel_id,
            s.stale_tag_id
        "#,
        limit
    )
    .fetch_all(pool)
    .await
}

pub async fn claim_cleared(
    pool: &PgPool,
    limit: i64,
) -> sqlx::Result<Vec<StaleTarget>> {
    sqlx::query_as!(
        StaleTarget,
        r#"
        WITH cleared AS (
            SELECT a.thread_id
            FROM support_thread_activity a
            WHERE a.staled_at IS NOT NULL
              AND a.since > a.staled_at
            ORDER BY a.thread_id
            LIMIT $1
            FOR UPDATE OF a SKIP LOCKED
        )
        UPDATE support_thread_activity a
        SET staled_at = NULL
        FROM cleared, support_settings s
        WHERE a.thread_id = cleared.thread_id
          AND s.guild_id = a.guild_id
        RETURNING
            a.thread_id,
            a.guild_id,
            s.support_channel_id,
            s.stale_tag_id
        "#,
        limit
    )
    .fetch_all(pool)
    .await
}
