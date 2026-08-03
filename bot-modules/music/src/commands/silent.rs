use std::collections::HashMap;

use serenity::all::{EditInteractionResponse, ResolvedValue};
use zayden_core::optional_option;

use super::MusicCtx;
use crate::error::{MusicError, Result};

pub(super) async fn run(
    ctx: &MusicCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    let requested: Option<bool> = optional_option(&mut options, "enabled");

    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::NotConnected)?;
    let (silenced, guild_enabled) = {
        let mut guard = player.lock().await;
        guard.silenced = requested.unwrap_or(!guard.silenced);
        (guard.silenced, guard.announce.enabled)
    };

    let content = match (silenced, guild_enabled) {
        (true, _) => "Now-playing announcements are silenced for this session.",
        (false, true) => "Now-playing announcements are back on.",
        (false, false) => {
            "Session silence cleared, but announcements stay off: this server \
             has `announce_now_playing` disabled."
        },
    };

    ctx.interaction
        .edit_response(ctx.http, EditInteractionResponse::new().content(content))
        .await?;

    Ok(())
}
