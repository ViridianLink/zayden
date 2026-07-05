use std::collections::HashMap;

use serenity::all::{EditInteractionResponse, ResolvedValue};
use zayden_core::required_option;

use super::MusicCtx;
use crate::error::{MusicError, Result};
use crate::track::LoopMode;

pub(super) async fn run(
    ctx: &MusicCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    let mode: &str = required_option(&mut options, "mode")?;
    let mode = match mode {
        "off" => LoopMode::Off,
        "track" => LoopMode::Track,
        "queue" => LoopMode::Queue,
        other => {
            return Err(MusicError::Internal(format!("unknown loop mode: {other}")));
        },
    };

    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::NothingPlaying)?;
    let mut guard = player.lock().await;
    guard.loop_mode = mode;
    drop(guard);

    ctx.interaction
        .edit_response(
            ctx.http,
            EditInteractionResponse::new()
                .content(format!("Loop mode set to {mode:?}.")),
        )
        .await?;

    Ok(())
}
