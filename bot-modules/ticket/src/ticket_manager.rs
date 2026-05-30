use async_trait::async_trait;
use serenity::all::{MessageId, RoleId};
use sqlx::{Database, FromRow, Pool};

#[async_trait]
pub trait TicketManager<Db: Database> {
    async fn get(
        pool: &Pool<Db>,
        id: impl Into<MessageId> + Send,
    ) -> sqlx::Result<TicketRow>;
    async fn delete(
        pool: &Pool<Db>,
        id: impl Into<MessageId> + Send,
    ) -> sqlx::Result<()>;
}

#[derive(FromRow)]
pub struct TicketRow {
    pub thread_id: i64,
    pub role_ids: Vec<i64>,
}

impl TicketRow {
    #[must_use]
    pub const fn message_id(&self) -> MessageId {
        MessageId::new(self.thread_id.cast_unsigned())
    }

    #[must_use]
    pub fn role_ids(&self) -> Vec<RoleId> {
        self.role_ids
            .iter()
            .copied()
            .map(|id| RoleId::new(id.cast_unsigned()))
            .collect()
    }
}
