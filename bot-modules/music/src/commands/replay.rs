use std::time::Duration;

use super::MusicCtx;
use super::seek::seek_to;
use crate::error::Result;

pub(super) async fn run(ctx: &MusicCtx<'_>) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    seek_to(ctx, Duration::ZERO).await
}
