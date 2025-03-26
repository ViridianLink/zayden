use serenity::all::{Context, VoiceState};
use sqlx::PgPool;

use crate::modules::temp_voice;
use crate::Result;

use super::Handler;

impl Handler {
    pub(super) async fn voice_state_update(
        ctx: &Context,
        new: VoiceState,
        pool: &PgPool,
    ) -> Result<()> {
        temp_voice::events::run(ctx, pool, &new).await?;

        Ok(())
    }
}
