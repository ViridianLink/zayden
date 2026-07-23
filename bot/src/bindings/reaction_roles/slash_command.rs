use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::CreateCommand;
use zayden_core::{HandlerError, InvocationCtx, ModuleCommand};

pub struct ReactionRoleCommand;

#[async_trait]
impl ModuleCommand for ReactionRoleCommand {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("reaction_role")
    }

    fn definition(&self) -> CreateCommand<'static> {
        reaction_roles::ReactionRoleCommand::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        reaction_roles::ReactionRoleCommand::run(
            &cx.ctx.http,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}
