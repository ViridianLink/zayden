use std::collections::HashMap;
use std::sync::Arc;

use serenity::all::{EditInteractionResponse, ResolvedOption, ResolvedValue};
use zayden_app::config::radio;
use zayden_app::entitlement::{EntitlementScope, Tier};
use zayden_core::{parse_options, parse_subcommand, required_option};

use super::MusicCtx;
use crate::error::{MusicError, Result};
use crate::radio::RADIO_TIER;
use crate::resolve::station_track;
use crate::{embeds, voice};

pub(super) async fn run(
    ctx: &MusicCtx<'_>,
    options: impl IntoIterator<Item = ResolvedOption<'_>>,
) -> Result<()> {
    let (name, sub_options) = parse_subcommand(options).map_err(MusicError::from)?;
    let options = parse_options(sub_options);

    match name {
        "play" => play(ctx, options).await,
        "stop" => stop(ctx).await,
        "list" => list(ctx).await,
        other => Err(MusicError::Internal(format!(
            "unexpected radio subcommand: {other}"
        ))),
    }
}

async fn require_tier(ctx: &MusicCtx<'_>) -> Result<()> {
    if RADIO_TIER == Tier::Free {
        return Ok(());
    }

    let scope = EntitlementScope::UserInGuild(
        ctx.interaction.user.id.get(),
        ctx.guild_id.get(),
    );

    if ctx.entitlements.allows(scope, RADIO_TIER).await {
        Ok(())
    } else {
        Err(MusicError::PremiumRequired)
    }
}

async fn play(
    ctx: &MusicCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    if ctx.radio_stations.is_empty() {
        return Err(MusicError::NoStationsConfigured);
    }

    require_tier(ctx).await?;

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    let id: &str = required_option(&mut options, "station")?;
    let station = radio::find(&ctx.radio_stations, id)
        .ok_or_else(|| MusicError::UnknownStation(id.to_string()))?;
    let station = Arc::new(station.clone());

    let request = ctx.session_request(&settings);
    voice::ensure_session(&ctx.songbird, &ctx.music, request).await?;

    let track = station_track(&station, ctx.interaction.user.id);

    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::NotConnected)?;
    let (old_handle, generation) = {
        let mut guard = player.lock().await;
        let old_handle = guard.current.as_ref().map(|now| now.handle.clone());
        guard.advance();
        guard.set_radio(Arc::clone(&station));
        (old_handle, guard.generation)
    };

    voice::stop_current_and_start(
        &ctx.playback(),
        ctx.guild_id,
        old_handle,
        Some(track),
        generation,
    )
    .await?;

    ctx.interaction
        .edit_response(
            ctx.http,
            EditInteractionResponse::new().embed(embeds::radio_embed(&station)),
        )
        .await?;

    Ok(())
}

async fn stop(ctx: &MusicCtx<'_>) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let settings = ctx.settings().await?;
    ctx.require_privileged(&settings)?;

    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::NotConnected)?;

    let stopped = {
        let mut guard = player.lock().await;
        guard.clear_radio().then(|| {
            let old_handle = guard.current.as_ref().map(|now| now.handle.clone());
            let next = guard.advance_queue();
            (old_handle, next, guard.generation)
        })
    };

    let message = match stopped {
        Some((old_handle, next, generation)) => {
            let resumed = next.is_some();
            voice::stop_current_and_start(
                &ctx.playback(),
                ctx.guild_id,
                old_handle,
                next,
                generation,
            )
            .await?;

            if resumed {
                "Stopped the radio. Playing the next queued track."
            } else {
                "Stopped the radio."
            }
        },
        None => "The radio isn't currently playing.",
    };

    ctx.interaction
        .edit_response(ctx.http, EditInteractionResponse::new().content(message))
        .await?;

    Ok(())
}

async fn list(ctx: &MusicCtx<'_>) -> Result<()> {
    ctx.interaction.defer_ephemeral(ctx.http).await?;

    ctx.interaction
        .edit_response(
            ctx.http,
            EditInteractionResponse::new()
                .embed(embeds::radio_list_embed(&ctx.radio_stations)),
        )
        .await?;

    Ok(())
}
