use std::collections::HashMap;
use std::time::Duration;

use serenity::all::ResolvedValue;
use zayden_core::required_option;

use super::MusicCtx;
use super::seek::{current_elapsed, seek_to};
use crate::error::Result;

pub(super) async fn run(
    ctx: &MusicCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    let secs: i64 = required_option(&mut options, "secs")?;
    let elapsed = current_elapsed(ctx).await?;
    let target = elapsed
        .saturating_sub(Duration::from_secs(u64::try_from(secs).unwrap_or(0)));

    seek_to(ctx, target).await
}
