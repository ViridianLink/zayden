use async_trait::async_trait;
use serenity::all::{ChannelId, GuildId};
use sqlx::{Database, FromRow, Pool};

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
    pub fn channel_id(&self) -> Option<ChannelId> {
        self.suggestions_channel_id
            .map(|id| ChannelId::new(id as u64))
    }

    pub fn review_channel_id(&self) -> Option<ChannelId> {
        self.review_channel_id.map(|id| ChannelId::new(id as u64))
    }
}
