use std::collections::HashMap;
use std::sync::Arc;

use serenity::all::{EditInteractionResponse, ResolvedValue};
use zayden_core::required_option;

use super::play::{enqueue, resolve_head, spawn_lazy_tail};
use super::MusicCtx;
use crate::error::Result;

pub(super) async fn run(
    ctx: &MusicCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    let query: &str = required_option(&mut options, "query")?;
    let (first, tail) = resolve_head(ctx, query).await?;
    let embed = enqueue(ctx, first, true).await?;

    ctx.interaction
        .edit_response(ctx.http, EditInteractionResponse::new().embed(embed))
        .await?;

    if let Some(tail) = tail {
        spawn_lazy_tail(
            Arc::clone(&ctx.http_owned),
            Arc::clone(&ctx.music),
            ctx.guild_id,
            tail,
        );
    }

    Ok(())
}
