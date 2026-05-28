use async_trait::async_trait;
use serenity::all::GuildId;
use sqlx::{PgPool, Postgres};
use suggestions::{SuggestionsGuildManager, SuggestionsGuildRow};
use zayden_app::config::ConfigStore;

pub mod slash_command;

pub use slash_command::FetchSuggestions;

use crate::sqlx_lib::GuildTable;

#[async_trait]
impl SuggestionsGuildManager<Postgres> for GuildTable {
    async fn get(
        pool: &PgPool,
        id: impl Into<GuildId> + Send,
    ) -> sqlx::Result<Option<SuggestionsGuildRow>> {
        let id = id.into();

        let Some(cfg) = ConfigStore::from_pool(pool.clone())
            .try_get(id.get() as i64)
            .await?
        else {
            return Ok(None);
        };

        Ok(Some(SuggestionsGuildRow {
            id: cfg.id,
            suggestions_channel_id: cfg.suggestions_channel_id,
            review_channel_id: cfg.review_channel_id,
        }))
    }
}
