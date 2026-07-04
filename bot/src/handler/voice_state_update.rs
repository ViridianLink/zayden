use serenity::all::{Context, VoiceState};
use sqlx::PgPool;
use tokio::sync::RwLock;

use super::Handler;
use crate::bindings::temp_voice;
use crate::{BotState, Result};

impl Handler {
    pub(super) async fn voice_state_update(
        ctx: &Context,
        new: &VoiceState,
        pool: &PgPool,
    ) -> Result<()> {
        temp_voice::events::run(ctx, pool, new).await?;

        ctx.data::<RwLock<BotState>>().read().await.music.occupancy().update(new);

        Ok(())
    }
}
