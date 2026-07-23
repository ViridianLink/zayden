use serenity::all::{ChannelId, GuildId};
use sqlx::{FromRow, PgPool};
use zayden_core::{as_i64, as_u64};

#[derive(FromRow)]
pub struct SuggestionsGuildRow {
    pub id: i64,
    pub suggestions_channel_id: Option<i64>,
    pub review_channel_id: Option<i64>,
}

impl SuggestionsGuildRow {
    #[must_use]
    pub fn channel_id(&self) -> Option<ChannelId> {
        self.suggestions_channel_id.map(|id| ChannelId::new(as_u64(id)))
    }

    #[must_use]
    pub fn review_channel_id(&self) -> Option<ChannelId> {
        self.review_channel_id.map(|id| ChannelId::new(as_u64(id)))
    }

    pub async fn get(pool: &PgPool, id: GuildId) -> sqlx::Result<Option<Self>> {
        sqlx::query_as!(
            Self,
            r#"
            SELECT guild_id AS id, suggestions_channel_id, review_channel_id
            FROM suggestions_settings
            WHERE guild_id = $1
            "#,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }
}
