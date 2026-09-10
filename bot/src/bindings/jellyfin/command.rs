use std::borrow::Cow;

use async_trait::async_trait;
use serenity::all::CreateCommand;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use super::runtime;

pub struct Jellyfin;

#[async_trait]
impl ModuleCommand for Jellyfin {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("jellyfin")
    }

    fn module(&self) -> Option<&'static str> {
        Some("jellyfin")
    }

    fn definition(&self) -> CreateCommand<'static> {
        ::jellyfin::Jellyfin::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let runtime = runtime(cx.ctx).await?;
        ::jellyfin::Jellyfin::run(cx, &runtime).await?;
        Ok(())
    }
}
