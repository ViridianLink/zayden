use std::collections::HashMap;

use serenity::all::{EditInteractionResponse, ResolvedValue};
use songbird::tracks::TrackHandle;

use super::MusicCtx;
use crate::error::{MusicError, Result};
use crate::track::ResolvedTrack;
use crate::{permissions, voice};

enum Outcome {
    Skipped {
        old_handle: Option<TrackHandle>,
        next: Option<ResolvedTrack>,
        generation: u64,
        forced: bool,
    },
    VoteRegistered {
        have: usize,
        needed: usize,
    },
}

pub(super) async fn run(
    ctx: &MusicCtx<'_>,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    ctx.interaction.defer(ctx.http).await?;

    let force =
        matches!(options.remove("force"), Some(ResolvedValue::Boolean(true)));

    let user_id = ctx.interaction.user.id;
    let bot_channel = ctx
        .music
        .occupancy()
        .channel_of(ctx.guild_id, ctx.bot_id)
        .ok_or(MusicError::NotConnected)?;
    if ctx.music.occupancy().channel_of(ctx.guild_id, user_id) != Some(bot_channel) {
        return Err(MusicError::UserNotInVoice);
    }

    let settings = ctx.settings().await?;
    let privileged = ctx.is_privileged(&settings);
    if force {
        ctx.require_privileged(&settings)?;
    }
    let listeners =
        ctx.music.occupancy().non_bot_count(ctx.guild_id, bot_channel, ctx.bot_id);

    let player = ctx.music.get(ctx.guild_id).ok_or(MusicError::NothingPlaying)?;

    let outcome = {
        let mut guard = player.lock().await;
        let now = guard.current.as_ref().ok_or(MusicError::NothingPlaying)?;
        let is_requester = now.track.requested_by.user_id == user_id;
        let alone = listeners <= 1;

        let can_skip_now = force || privileged || is_requester || alone || {
            guard.skip_votes.insert(user_id);
            guard.skip_votes.len() >= permissions::vote_threshold(listeners)
        };

        if can_skip_now {
            let old_handle = guard.current.as_ref().map(|now| now.handle.clone());
            let next = guard.advance_queue();
            Outcome::Skipped {
                old_handle,
                next,
                generation: guard.generation,
                forced: force,
            }
        } else {
            Outcome::VoteRegistered {
                have: guard.skip_votes.len(),
                needed: permissions::vote_threshold(listeners),
            }
        }
    };

    let message = match outcome {
        Outcome::Skipped { old_handle, next, generation, forced } => {
            let started_next = next.is_some();
            voice::stop_current_and_start(
                &ctx.songbird,
                &ctx.music,
                &ctx.resolver,
                ctx.guild_id,
                old_handle,
                next,
                generation,
            )
            .await?;
            let verb = if forced { "Force-skipped" } else { "Skipped" };
            if started_next {
                format!("{verb}. Playing the next track.")
            } else {
                format!("{verb}. The queue is now empty.")
            }
        },
        Outcome::VoteRegistered { have, needed } => {
            format!("Vote to skip: {have}/{needed}")
        },
    };

    ctx.interaction
        .edit_response(ctx.http, EditInteractionResponse::new().content(message))
        .await?;

    Ok(())
}
