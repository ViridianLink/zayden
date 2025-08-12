use async_trait::async_trait;
use serenity::all::{Context, CreateCommand, GuildId};
use sqlx::{PgPool, Postgres};
use suggestions::{SuggestionsGuildManager, SuggestionsGuildRow};
use zayden_core::ApplicationCommand;

pub mod slash_command;

pub use slash_command::FetchSuggestions;

use crate::sqlx_lib::GuildTable;

pub fn register(ctx: &Context) -> CreateCommand<'_> {
    FetchSuggestions::register(ctx).unwrap()
}

#[async_trait]
impl SuggestionsGuildManager<Postgres> for GuildTable {
    async fn get(
        pool: &PgPool,
        id: impl Into<GuildId> + Send,
    ) -> sqlx::Result<Option<SuggestionsGuildRow>> {
        let id = id.into();

        let row = sqlx::query_as!(
            SuggestionsGuildRow,
            r#"SELECT id, suggestions_channel_id, review_channel_id FROM guilds WHERE id = $1"#,
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
        .unwrap();

        Ok(row)
    }
}
