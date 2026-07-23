use serenity::all::{GenericChannelId, GuildId, MessageId, RoleId};
use sqlx::PgPool;
use sqlx::postgres::PgQueryResult;
use sqlx::prelude::FromRow;
use zayden_core::{as_i64, as_u64};

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
        GuildId::new(as_u64(self.guild_id))
    }

    #[must_use]
    pub const fn channel_id(&self) -> GenericChannelId {
        GenericChannelId::new(as_u64(self.channel_id))
    }

    #[must_use]
    pub const fn message_id(&self) -> MessageId {
        MessageId::new(as_u64(self.message_id))
    }

    #[must_use]
    pub const fn role_id(&self) -> RoleId {
        RoleId::new(as_u64(self.role_id))
    }

    pub async fn create(
        pool: &PgPool,
        guild_id: GuildId,
        channel_id: GenericChannelId,
        message_id: MessageId,
        role_id: RoleId,
        emoji: &str,
    ) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "INSERT INTO
    reaction_roles (guild_id, channel_id, message_id, role_id, emoji)
VALUES
    ($1, $2, $3, $4, $5)",
            as_i64(guild_id.get()),
            as_i64(channel_id.get()),
            as_i64(message_id.get()),
            as_i64(role_id.get()),
            emoji
        )
        .execute(pool)
        .await
    }

    pub async fn rows(
        pool: &PgPool,
        guild_id: GuildId,
    ) -> sqlx::Result<Vec<Self>> {
        sqlx::query_as!(
            Self,
            "SELECT * FROM reaction_roles WHERE guild_id = $1",
            as_i64(guild_id.get())
        )
        .fetch_all(pool)
        .await
    }

    pub async fn row(
        pool: &PgPool,
        message_id: MessageId,
        emoji: &str,
    ) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            "SELECT * FROM reaction_roles WHERE message_id = $1 AND emoji = $2",
            as_i64(message_id.get()),
            emoji
        )
        .fetch_optional(pool)
        .await
    }

    pub async fn delete(
        pool: &PgPool,
        guild_id: GuildId,
        channel_id: GenericChannelId,
        message_id: MessageId,
        emoji: &str,
    ) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "DELETE FROM reaction_roles WHERE guild_id = $1 AND channel_id = $2 AND message_id = $3 AND emoji = $4",
            as_i64(guild_id.get()),
            as_i64(channel_id.get()),
            as_i64(message_id.get()),
            emoji
        )
        .execute(pool)
        .await
    }
}
