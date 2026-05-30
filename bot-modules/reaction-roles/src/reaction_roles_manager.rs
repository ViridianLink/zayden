use serenity::all::{GenericChannelId, GuildId, MessageId, RoleId};
use serenity::async_trait;
use sqlx::{Database, FromRow, Pool};

#[async_trait]
pub trait ReactionRolesManager<Db: Database> {
    async fn create(
        pool: &Pool<Db>,
        guild_id: impl Into<GuildId> + Send,
        channel_id: impl Into<GenericChannelId> + Send,
        message_id: impl Into<MessageId> + Send,
        role_id: impl Into<RoleId> + Send,
        emoji: &str,
    ) -> sqlx::Result<Db::QueryResult>;

    async fn rows(
        pool: &Pool<Db>,
        guild_id: impl Into<GuildId> + Send,
    ) -> sqlx::Result<Vec<ReactionRole>>;

    async fn row(
        pool: &Pool<Db>,
        message_id: impl Into<MessageId> + Send,
        emoji: &str,
    ) -> sqlx::Result<Option<ReactionRole>>;

    async fn delete(
        pool: &Pool<Db>,
        guild_id: impl Into<GuildId> + Send,
        channel_id: impl Into<GenericChannelId> + Send,
        message_id: impl Into<MessageId> + Send,
        emoji: &str,
    ) -> sqlx::Result<Db::QueryResult>;
}

#[derive(FromRow)]
pub struct ReactionRole {
    pub id: i32,
    pub guild_id: i64,
    pub channel_id: i64,
    pub message_id: i64,
    pub role_id: i64,
    pub emoji: String,
}

impl ReactionRole {
    #[must_use]
    pub const fn guild_id(&self) -> GuildId {
        GuildId::new(self.guild_id.cast_unsigned())
    }

    #[must_use]
    pub const fn channel_id(&self) -> GenericChannelId {
        GenericChannelId::new(self.channel_id.cast_unsigned())
    }

    #[must_use]
    pub const fn message_id(&self) -> MessageId {
        MessageId::new(self.message_id.cast_unsigned())
    }

    #[must_use]
    pub const fn role_id(&self) -> RoleId {
        RoleId::new(self.role_id.cast_unsigned())
    }
}
