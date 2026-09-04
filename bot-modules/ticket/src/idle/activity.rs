use serenity::all::{GuildId, RoleId, ThreadId, UserId};
use sqlx::PgPool;
use zayden_core::{as_i64, as_u64};

use crate::idle::Ball;

pub struct ThreadActivity {
    pub thread_id: i64,
    pub guild_id: i64,
    pub op_id: i64,
    pub helper_id: Option<i64>,
    pub waiting_on_helper: bool,
}

impl ThreadActivity {
    #[must_use]
    pub const fn op(&self) -> UserId {
        UserId::new(as_u64(self.op_id))
    }

    #[must_use]
    pub const fn ball(&self) -> Ball {
        Ball::from_column(self.waiting_on_helper)
    }

    pub async fn insert(
        pool: &PgPool,
        guild_id: GuildId,
        thread_id: ThreadId,
        op: UserId,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "INSERT INTO support_thread_activity (thread_id, guild_id, op_id) \
             VALUES ($1, $2, $3) ON CONFLICT (thread_id) DO NOTHING",
            as_i64(thread_id.get()),
            as_i64(guild_id.get()),
            as_i64(op.get())
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn active(
        pool: &PgPool,
        thread_id: ThreadId,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            "SELECT thread_id, guild_id, op_id, helper_id, waiting_on_helper \
             FROM support_thread_activity \
             WHERE thread_id = $1 AND NOT paused",
            as_i64(thread_id.get())
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn track(
        pool: &PgPool,
        thread_id: ThreadId,
        author: UserId,
        roles: &[RoleId],
    ) -> sqlx::Result<()> {
        let role_ids =
            roles.iter().map(|role| as_i64(role.get())).collect::<Vec<_>>();

        sqlx::query!(
            "UPDATE support_thread_activity a \
             SET waiting_on_helper = (a.op_id = $2), \
                 helper_id = CASE WHEN a.op_id <> $2 THEN $2 ELSE a.helper_id END, \
                 since = now(), \
                 nudged_at = NULL \
             WHERE a.thread_id = $1 \
               AND NOT a.paused \
               AND (a.op_id = $2 \
                    OR EXISTS (SELECT 1 FROM guild_support_roles r \
                               WHERE r.guild_id = a.guild_id \
                                 AND r.role_id = ANY($3::bigint[])))",
            as_i64(thread_id.get()),
            as_i64(author.get()),
            &role_ids
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn pause(pool: &PgPool, thread_id: ThreadId) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE support_thread_activity SET paused = TRUE \
             WHERE thread_id = $1",
            as_i64(thread_id.get())
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn resume(pool: &PgPool, thread_id: ThreadId) -> sqlx::Result<()> {
        sqlx::query!(
            "UPDATE support_thread_activity \
             SET paused = FALSE, waiting_on_helper = TRUE, since = now(), \
                 nudged_at = NULL \
             WHERE thread_id = $1",
            as_i64(thread_id.get())
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete(pool: &PgPool, thread_id: ThreadId) -> sqlx::Result<()> {
        sqlx::query!(
            "DELETE FROM support_thread_activity WHERE thread_id = $1",
            as_i64(thread_id.get())
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
