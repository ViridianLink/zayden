use std::borrow::Cow;
use std::sync::Arc;

use async_trait::async_trait;
use music::commands::{Command as MusicCommand, MusicCtx, MusicServices};
use serenity::all::{CreateCommand, UserId};
use tokio::sync::RwLock;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;

pub struct Music;

#[async_trait]
impl ModuleCommand for Music {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("music")
    }

    fn definition(&self) -> CreateCommand<'static> {
        MusicCommand::register()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();

        let data = cx.ctx.data::<RwLock<BotState>>();
        let guard = data.read().await;
        let songbird = Arc::clone(&guard.songbird);
        let music = Arc::clone(&guard.music);
        let resolver = Arc::clone(&guard.music_resolver);
        drop(guard);

        let settings = Arc::clone(&cx.app.settings.music);

        let music_ctx = MusicCtx::new(
            &cx.ctx.http,
            cx.interaction,
            UserId::new(cx.app.zayden_id),
            MusicServices {
                http: Arc::clone(&cx.ctx.http),
                songbird,
                music,
                resolver,
                settings,
                entitlements: Arc::clone(&cx.app.entitlements),
            },
        )?;

        MusicCommand::run(&music_ctx, options).await?;
        Ok(())
    }
}
