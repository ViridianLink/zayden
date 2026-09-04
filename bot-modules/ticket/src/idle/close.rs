use serenity::all::{ChannelId, ForumTagId, GuildId, ThreadId, UserId};
use sqlx::PgPool;
use zayden_core::as_u64;

pub struct DueClose {
    pub thread_id: i64,
    pub guild_id: i64,
    pub op_id: i64,
    pub since: jiff_sqlx::Timestamp,
    pub support_channel_id: Option<i64>,
    pub closed_tag_id: Option<i64>,
}

impl DueClose {
    #[must_use]
    pub const fn thread(&self) -> ThreadId {
        ThreadId::new(as_u64(self.thread_id))
    }

    #[must_use]
    pub const fn guild(&self) -> GuildId {
        GuildId::new(as_u64(self.guild_id))
    }

    #[must_use]
    pub const fn op(&self) -> UserId {
        UserId::new(as_u64(self.op_id))
    }

    #[must_use]
    pub fn support_channel(&self) -> Option<ChannelId> {
        self.support_channel_id.map(|id| ChannelId::new(as_u64(id)))
    }

    #[must_use]
    pub fn closed_tag(&self) -> Option<ForumTagId> {
        self.closed_tag_id.map(|id| ForumTagId::new(as_u64(id)))
    }

    #[must_use]
    pub fn since(&self) -> i64 {
        self.since.to_jiff().as_second()
    }
}

pub async fn claim_due(pool: &PgPool, limit: i64) -> sqlx::Result<Vec<DueClose>> {
    sqlx::query_as!(
        DueClose,
        r#"
        WITH due AS (
            SELECT a.thread_id
            FROM support_thread_activity a
            JOIN support_settings s ON s.guild_id = a.guild_id
            WHERE s.idle_enabled
              AND s.idle_close_enabled
              AND NOT a.paused
              AND NOT a.waiting_on_helper
              AND a.nudged_at IS NOT NULL
              AND a.nudged_at < now() - (s.idle_close_after_secs * interval '1 second')
            ORDER BY a.guild_id, a.nudged_at
            LIMIT $1
            FOR UPDATE OF a SKIP LOCKED
        )
        UPDATE support_thread_activity a
        SET paused = TRUE
        FROM due, support_settings s
        WHERE a.thread_id = due.thread_id
          AND s.guild_id = a.guild_id
        RETURNING
            a.thread_id,
            a.guild_id,
            a.op_id,
            a.since AS "since: jiff_sqlx::Timestamp",
            s.support_channel_id,
            s.closed_tag_id
        "#,
        limit
    )
    .fetch_all(pool)
    .await
}
