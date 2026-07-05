use std::collections::HashMap;
use std::sync::Arc;

use serenity::all::{EditInteractionResponse, ResolvedValue};
use zayden_core::required_option;

use super::play::{resolve_head, spawn_lazy_tail};
use super::MusicCtx;
use crate::embeds;
use crate::error::{MusicError, Result};
use crate::voice;

pub(super) async fn run(
    ctx: &MusicCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    let query: &str = required_option(&mut options, "query")?;
    let (first, tail) = resolve_head(ctx, query).await?;

    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::NotConnected)?;
    let (old_handle, generation) = {
        let mut guard = player.lock().await;
        let old_handle = guard.current.as_ref().map(|now| now.handle.clone());
        guard.advance();
        (old_handle, guard.generation)
    };

    voice::stop_current_and_start(
        &ctx.songbird,
        &ctx.music,
        &ctx.resolver,
        ctx.guild_id,
        old_handle,
        Some(first.clone()),
        generation,
    )
    .await?;

    let guard = player.lock().await;
    let embed = guard.current.as_ref().map_or_else(
        || embeds::queued_embed(&first, 1),
        |now| embeds::now_playing_embed(now, guard.loop_mode),
    );
    drop(guard);

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
