use std::time::Duration;

use super::seek::seek_to;
use super::MusicCtx;
use crate::error::Result;

pub(super) async fn run(ctx: &MusicCtx<'_>) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    seek_to(ctx, Duration::ZERO).await
}
