use serenity::all::{MessageId, RoleId};
use sqlx::{FromRow, PgPool};
use zayden_core::{as_i64, as_u64};

#[derive(FromRow)]
pub struct TicketRow {
    pub thread_id: i64,
    pub role_ids: Vec<i64>,
}

impl TicketRow {
    #[must_use]
    pub const fn message_id(&self) -> MessageId {
        MessageId::new(as_u64(self.thread_id))
    }

    #[must_use]
    pub fn role_ids(&self) -> Vec<RoleId> {
        self.role_ids.iter().copied().map(|id| RoleId::new(as_u64(id))).collect()
    }

    pub async fn get(
        pool: &PgPool,
        id: impl Into<MessageId> + Send,
    ) -> sqlx::Result<Self> {
        sqlx::query_as!(
            Self,
            r#"SELECT thread_id, COALESCE(
                        (SELECT array_agg(role_id) FROM ticket_roles WHERE ticket_id = t.thread_id),
                        ARRAY[]::bigint[]
                    ) AS "role_ids!" FROM tickets t WHERE thread_id = $1"#,
            as_i64(id.into().get())
        )
        .fetch_one(pool)
        .await
    }

    pub async fn delete(
        pool: &PgPool,
        id: impl Into<MessageId> + Send,
    ) -> sqlx::Result<()> {
        sqlx::query!(
            "DELETE FROM tickets WHERE thread_id = $1",
            as_i64(id.into().get())
        )
        .execute(pool)
        .await?;

        Ok(())
    }
}
