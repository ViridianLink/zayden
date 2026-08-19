use std::borrow::Cow;

use async_trait::async_trait;
use family::commands::Command as FamilyCommand;
use serenity::all::CreateCommand;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

pub struct Family;

#[async_trait]
impl ModuleCommand for Family {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("family")
    }

    fn module(&self) -> Option<&'static str> {
        Some("family")
    }

    fn definition(&self) -> CreateCommand<'static> {
        FamilyCommand::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        FamilyCommand::run(cx).await?;
        Ok(())
    }
}
