use std::borrow::Cow;

use async_trait::async_trait;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::sqlx_lib::GuildTable;

pub struct FetchSuggestions;

#[async_trait]
impl ModuleCommand for FetchSuggestions {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("fetch_suggestions")
    }

    fn definition(&self) -> serenity::all::CreateCommand<'static> {
        suggestions::FetchSuggestions::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        suggestions::FetchSuggestions::run::<sqlx::Postgres, GuildTable>(
            &cx.ctx.http,
            cx.interaction,
            cx.interaction.data.options(),
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}
