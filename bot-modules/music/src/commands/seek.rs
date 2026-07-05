use std::collections::HashMap;
use std::time::{Duration, Instant};

use serenity::all::{EditInteractionResponse, ResolvedValue};
use zayden_core::required_option;

use super::MusicCtx;
use crate::embeds::{format_duration, parse_timestamp};
use crate::error::{MusicError, Result};

pub(super) async fn run(
    ctx: &MusicCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    let timestamp: &str = required_option(&mut options, "timestamp")?;
    let target = parse_timestamp(timestamp).ok_or(MusicError::InvalidTimestamp)?;

    seek_to(ctx, target).await
}

pub(super) async fn current_elapsed(ctx: &MusicCtx<'_>) -> Result<Duration> {
    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::NothingPlaying)?;
    let guard = player.lock().await;
    let now = guard.current.as_ref().ok_or(MusicError::NothingPlaying)?;
    let elapsed = now.started_at.elapsed();
    drop(guard);
    Ok(elapsed)
}

pub(super) async fn seek_to(ctx: &MusicCtx<'_>, target: Duration) -> Result<()> {
    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::NothingPlaying)?;

    let (handle, is_live, duration) = {
        let guard = player.lock().await;
        let now = guard.current.as_ref().ok_or(MusicError::NothingPlaying)?;
        let info = (now.handle.clone(), now.track.is_live, now.track.duration);
        drop(guard);
        info
    };

    if is_live {
        return Err(MusicError::SeekOnLiveStream);
    }

    let clamped = duration.map_or(target, |d| target.min(d));

    handle
        .seek_async(clamped)
        .await
        .map_err(|e| MusicError::Songbird(e.to_string()))?;

    {
        let mut guard = player.lock().await;
        if let Some(now) = guard.current.as_mut() {
            now.started_at =
                Instant::now().checked_sub(clamped).unwrap_or_else(Instant::now);
        }
    }

    ctx.interaction
        .edit_response(
            ctx.http,
            EditInteractionResponse::new()
                .content(format!("Seeked to `{}`.", format_duration(clamped))),
        )
        .await?;

    Ok(())
}
