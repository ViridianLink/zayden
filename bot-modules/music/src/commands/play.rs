use std::collections::HashMap;
use std::sync::Arc;

use serenity::all::{
    CreateEmbed,
    EditInteractionResponse,
    GuildId,
    Http,
    ResolvedValue,
};
use zayden_core::required_option;

use super::MusicCtx;
use crate::error::{MusicError, Result};
use crate::manager::MusicManager;
use crate::resolve::{LazyTail, SourceQuery};
use crate::track::ResolvedTrack;
use crate::{embeds, voice};

const PLAYLIST_CAP: usize = 500;

pub(super) async fn run(
    ctx: &MusicCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let query: &str = required_option(&mut options, "query")?;
    let (first, tail) = resolve_head(ctx, query).await?;
    let embed = enqueue(ctx, first, false).await?;

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

pub(super) async fn resolve_head(
    ctx: &MusicCtx<'_>,
    query: &str,
) -> Result<(ResolvedTrack, Option<LazyTail>)> {
    let user_id = ctx.interaction.user.id;

    let settings = ctx.settings().await?;
    let request = ctx.session_request(&settings);
    voice::ensure_session(&ctx.songbird, &ctx.music, request).await?;

    let source_query = SourceQuery::new(query);
    let mut resolution = ctx.resolver.resolve(&source_query, user_id).await?;
    if resolution.head.is_empty() {
        return Err(MusicError::NoResults);
    }
    let first = resolution.head.remove(0);

    Ok((first, resolution.tail.take()))
}

pub(super) async fn enqueue(
    ctx: &MusicCtx<'_>,
    track: ResolvedTrack,
    at_top: bool,
) -> Result<CreateEmbed<'static>> {
    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::NotConnected)?;

    let (should_start, generation, position) = {
        let mut guard = player.lock().await;
        let should_start = guard.current.is_none();
        if at_top {
            guard.queue.insert_top(track.clone());
        } else {
            guard.queue.push(track.clone());
        }
        let position = if at_top { 1 } else { guard.queue.len() };
        (should_start, guard.generation, position)
    };

    if !should_start {
        return Ok(embeds::queued_embed(&track, position));
    }

    let next = {
        let mut guard = player.lock().await;
        guard.queue.pop_front()
    };
    if let Some(next) = next {
        voice::start_playback(
            &ctx.songbird,
            &ctx.music,
            &ctx.resolver,
            ctx.guild_id,
            generation,
            next,
        )
        .await?;
    }

    let guard = player.lock().await;
    let embed = guard.current.as_ref().map_or_else(
        || embeds::queued_embed(&track, 1),
        |now| embeds::now_playing_embed(now, guard.loop_mode),
    );
    drop(guard);

    Ok(embed)
}

pub(super) fn spawn_lazy_tail(
    http: Arc<Http>,
    music: Arc<MusicManager>,
    guild_id: GuildId,
    tail: LazyTail,
) {
    tokio::spawn(async move {
        let Ok(tracks) = tail.await else {
            return;
        };
        let Some(player) = music.get(guild_id) else {
            return;
        };

        let (added, truncated, text_channel) = {
            let mut guard = player.lock().await;
            let remaining_capacity = PLAYLIST_CAP.saturating_sub(guard.queue.len());
            let truncated = tracks.len() > remaining_capacity;
            let mut added = 0;
            for track in tracks.into_iter().take(remaining_capacity) {
                guard.queue.push(track);
                added += 1;
            }
            (added, truncated, guard.text_channel)
        };

        if truncated {
            let notice = MusicError::PlaylistTruncated { max: PLAYLIST_CAP };
            let _ = text_channel
                .say(
                    &http,
                    format!("{notice} ({added} tracks queued from this playlist)."),
                )
                .await;
        }
    });
}
