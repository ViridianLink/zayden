use serenity::all::{RoleId, ThreadId, UserId};
use sqlx::PgPool;
use zayden_core::as_u64;

use crate::idle::Ball;

pub struct DueNudge {
    pub thread_id: i64,
    pub op_id: i64,
    pub helper_id: Option<i64>,
    pub waiting_on_helper: bool,
    pub since: jiff_sqlx::Timestamp,
    pub support_role_ids: Vec<i64>,
}

impl DueNudge {
    #[must_use]
    pub const fn thread(&self) -> ThreadId {
        ThreadId::new(as_u64(self.thread_id))
    }

    #[must_use]
    pub const fn op(&self) -> UserId {
        UserId::new(as_u64(self.op_id))
    }

    #[must_use]
    pub fn helper(&self) -> Option<UserId> {
        self.helper_id.map(|id| UserId::new(as_u64(id)))
    }

    #[must_use]
    pub const fn ball(&self) -> Ball {
        Ball::from_column(self.waiting_on_helper)
    }

    #[must_use]
    pub fn support_roles(&self) -> Vec<RoleId> {
        self.support_role_ids.iter().map(|id| RoleId::new(as_u64(*id))).collect()
    }

    #[must_use]
    pub fn since(&self) -> i64 {
        self.since.to_jiff().as_second()
    }
}

pub async fn claim_due(pool: &PgPool, limit: i64) -> sqlx::Result<Vec<DueNudge>> {
    sqlx::query_as!(
        DueNudge,
        r#"
        WITH due AS (
            SELECT a.thread_id
            FROM support_thread_activity a
            JOIN support_settings s ON s.guild_id = a.guild_id
            WHERE s.idle_enabled
              AND NOT a.paused
              AND a.nudged_at IS NULL
              AND a.since < now() - (s.idle_after_secs * interval '1 second')
            ORDER BY a.guild_id, a.since
            LIMIT $1
            FOR UPDATE OF a SKIP LOCKED
        )
        UPDATE support_thread_activity a
        SET nudged_at = now()
        FROM due
        WHERE a.thread_id = due.thread_id
        RETURNING
            a.thread_id,
            a.op_id,
            a.helper_id,
            a.waiting_on_helper,
            a.since AS "since: jiff_sqlx::Timestamp",
            COALESCE(
                (SELECT array_agg(r.role_id ORDER BY r.role_id)
                 FROM guild_support_roles r
                 WHERE r.guild_id = a.guild_id),
                ARRAY[]::bigint[]
            ) AS "support_role_ids!"
        "#,
        limit
    )
    .fetch_all(pool)
    .await
}

pub async fn gc(pool: &PgPool) -> sqlx::Result<u64> {
    let deleted = sqlx::query!(
        "DELETE FROM support_thread_activity \
         WHERE since < now() - interval '60 days'"
    )
    .execute(pool)
    .await?
    .rows_affected();

    Ok(deleted)
}
