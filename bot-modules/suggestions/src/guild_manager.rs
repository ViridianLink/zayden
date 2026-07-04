use async_trait::async_trait;
use serenity::all::{ChannelId, GuildId};
use sqlx::{Database, FromRow, PgPool, Pool, Postgres};
use zayden_core::{as_i64, as_u64};

#[async_trait]
pub trait SuggestionsGuildManager<Db: Database> {
    async fn get(
        pool: &Pool<Db>,
        id: impl Into<GuildId> + Send,
    ) -> sqlx::Result<Option<SuggestionsGuildRow>>;
}

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
}

pub struct GuildTable;

#[async_trait]
impl SuggestionsGuildManager<Postgres> for GuildTable {
    async fn get(
        pool: &PgPool,
        id: impl Into<GuildId> + Send,
    ) -> sqlx::Result<Option<SuggestionsGuildRow>> {
        let id = id.into();

        sqlx::query_as!(
            SuggestionsGuildRow,
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
