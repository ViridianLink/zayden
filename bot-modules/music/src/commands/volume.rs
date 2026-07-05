use std::collections::HashMap;

use serenity::all::{EditInteractionResponse, ResolvedValue};

use super::MusicCtx;
use crate::error::{MusicError, Result};

pub(super) async fn run(
    ctx: &MusicCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let Some(ResolvedValue::Integer(requested)) = options.remove("volume") else {
        let settings = ctx.settings().await?;
        let current = match ctx.music.get(ctx.guild_id) {
            Some(player) => player.lock().await.volume,
            None => u8::try_from(settings.default_volume).unwrap_or(100),
        };
        ctx.interaction
            .edit_response(
                ctx.http,
                EditInteractionResponse::new().content(format!("Volume: {current}%")),
            )
            .await?;
        return Ok(());
    };

    if !(0..=100).contains(&requested) {
        return Err(MusicError::VolumeOutOfRange);
    }
    let volume = u8::try_from(requested).unwrap_or(100);

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    if let Some(player) = ctx.music.get(ctx.guild_id) {
        let mut guard = player.lock().await;
        guard.volume = volume;
        if let Some(now) = guard.current.as_ref() {
            now.handle
                .set_volume(f32::from(volume) / 100.0)
                .map_err(|e| MusicError::Songbird(e.to_string()))?;
        }
    }

    ctx.settings
        .update(zayden_core::as_i64(ctx.guild_id.get()), |row| {
            row.default_volume = i16::from(volume);
        })
        .await?;

    ctx.interaction
        .edit_response(
            ctx.http,
            EditInteractionResponse::new().content(format!("Volume set to {volume}%.")),
        )
        .await?;

    Ok(())
}
