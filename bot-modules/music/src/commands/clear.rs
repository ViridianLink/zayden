use std::collections::HashMap;

use serenity::all::{EditInteractionResponse, ResolvedValue};
use zayden_core::optional_option;

use super::MusicCtx;
use crate::error::{MusicError, Result};
use crate::queue::ClearMode;

pub(super) async fn run(
    ctx: &MusicCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    let raw: Option<&str> = optional_option(&mut options, "mode");
    let mode = ClearMode::parse(raw).ok_or_else(|| {
        MusicError::Internal(format!("unexpected clear mode: {raw:?}"))
    })?;

    let voice_members = match mode {
        ClearMode::Left => {
            let bot_channel = ctx
                .music
                .occupancy()
                .channel_of(ctx.guild_id, ctx.bot_id)
                .ok_or(MusicError::NotConnected)?;
            Some(ctx.music.occupancy().members_in_channel(ctx.guild_id, bot_channel))
        },
        ClearMode::All | ClearMode::Duplicates => None,
    };

    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::QueueEmpty)?;
    let mut guard = player.lock().await;
    let message = match mode {
        ClearMode::All => {
            guard.queue.clear();
            "Cleared the queue.".to_string()
        },
        ClearMode::Duplicates => {
            format!("Removed {} duplicate track(s).", guard.queue.dedupe())
        },
        ClearMode::Left => {
            let removed = voice_members
                .as_ref()
                .map_or(0, |members| guard.queue.cleanup(members));
            format!(
                "Removed {removed} track(s) requested by members who left voice."
            )
        },
    };
    drop(guard);

    ctx.interaction
        .edit_response(ctx.http, EditInteractionResponse::new().content(message))
        .await?;

    Ok(())
}
