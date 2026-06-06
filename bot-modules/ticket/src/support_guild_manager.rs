use async_trait::async_trait;
use serenity::all::{ChannelId, GuildId, RoleId};
use sqlx::{Database, FromRow, Pool};
use zayden_core::as_u64;

#[async_trait]
pub trait TicketGuildManager<Db: Database> {
    async fn get(
        pool: &Pool<Db>,
        id: impl Into<GuildId> + Send,
    ) -> sqlx::Result<Option<TicketGuildRow>>;

    async fn update_thread_id(
        pool: &Pool<Db>,
        id: impl Into<GuildId> + Send,
    ) -> sqlx::Result<()>;
}

#[derive(FromRow)]
pub struct TicketGuildRow {
    pub id: i64,
    pub thread_id: i32,
    pub support_channel_id: Option<i64>,
    pub support_role_ids: Vec<i64>,
    pub faq_channel_id: Option<i64>,
}

impl TicketGuildRow {
    #[must_use]
    pub fn channel_id(&self) -> Option<ChannelId> {
        self.support_channel_id.map(|id| ChannelId::new(as_u64(id)))
    }

    #[must_use]
    pub fn role_ids(&self) -> Vec<RoleId> {
        self.support_role_ids.iter().map(|&id| RoleId::new(as_u64(id))).collect()
    }

    #[must_use]
    pub fn faq_channel_id(&self) -> Option<ChannelId> {
        self.faq_channel_id.map(|id| ChannelId::new(as_u64(id)))
    }
}
