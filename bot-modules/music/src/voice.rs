use std::sync::Arc;
use std::time::{Duration, Instant};

use serenity::all::{ChannelId, GenericChannelId, GuildId, UserId};
use songbird::id::ChannelId as SongbirdChannelId;
use songbird::tracks::TrackHandle;
use songbird::{Call, Event, Songbird, TrackEvent};
use tokio::sync::Mutex;

use crate::error::{MusicError, Result};
use crate::events::{InactivityCheck, TrackEndNotifier};
use crate::manager::MusicManager;
use crate::player::NowPlaying;
use crate::resolve::TrackResolver;
use crate::track::ResolvedTrack;

pub async fn join<C>(
    songbird: &Arc<Songbird>,
    guild_id: GuildId,
    channel_id: C,
) -> Result<Arc<Mutex<Call>>>
where
    C: Into<SongbirdChannelId>,
{
    songbird
        .join(guild_id, channel_id)
        .await
        .map_err(|e| MusicError::Songbird(e.to_string()))
}

pub async fn leave(songbird: &Arc<Songbird>, guild_id: GuildId) -> Result<()> {
    songbird.leave(guild_id).await.map_err(|e| MusicError::Songbird(e.to_string()))
}

#[must_use]
pub fn get_call(
    songbird: &Arc<Songbird>,
    guild_id: GuildId,
) -> Option<Arc<Mutex<Call>>> {
    songbird.get(guild_id)
}

pub struct SessionRequest {
    pub guild_id: GuildId,
    pub user_id: UserId,
    pub bot_id: UserId,
    pub text_channel: GenericChannelId,
    pub default_volume: u8,
    pub auto_disconnect_secs: u64,
    pub stay_connected: bool,
}

pub async fn ensure_session(
    songbird: &Arc<Songbird>,
    music: &Arc<MusicManager>,
    request: SessionRequest,
) -> Result<(ChannelId, Arc<Mutex<Call>>)> {
    let channel_id = music
        .occupancy()
        .channel_of(request.guild_id, request.user_id)
        .ok_or(MusicError::UserNotInVoice)?;

    let call = join(songbird, request.guild_id, channel_id).await?;
    let player = music.get_or_create_player(
        request.guild_id,
        request.text_channel,
        request.default_volume,
    );

    let mut guard = player.lock().await;
    if !guard.periodic_registered {
        let mut call_guard = call.lock().await;
        call_guard.add_global_event(
            Event::Periodic(Duration::from_secs(30), None),
            InactivityCheck {
                guild_id: request.guild_id,
                channel_id,
                bot_id: request.bot_id,
                music: Arc::clone(music),
                songbird: Arc::clone(songbird),
                auto_disconnect_secs: request.auto_disconnect_secs,
                stay_connected: request.stay_connected,
            },
        );
        drop(call_guard);
        guard.periodic_registered = true;
    }
    drop(guard);

    Ok((channel_id, call))
}

pub async fn start_playback(
    songbird: &Arc<Songbird>,
    music: &Arc<MusicManager>,
    resolver: &Arc<dyn TrackResolver>,
    guild_id: GuildId,
    generation: u64,
    track: ResolvedTrack,
) -> Result<()> {
    let call = get_call(songbird, guild_id).ok_or(MusicError::NotConnected)?;
    let input = resolver.stream(&track).await?;

    let handle = {
        let mut call_guard = call.lock().await;
        call_guard.play_input(input)
    };

    handle
        .add_event(Event::Track(TrackEvent::End), TrackEndNotifier {
            guild_id,
            generation,
            music: Arc::clone(music),
            songbird: Arc::clone(songbird),
            resolver: Arc::clone(resolver),
        })
        .map_err(|e| MusicError::Songbird(e.to_string()))?;

    if let Some(player) = music.get(guild_id) {
        let mut guard = player.lock().await;
        if guard.generation == generation {
            guard.current =
                Some(NowPlaying { track, handle, started_at: Instant::now() });
        }
    }

    Ok(())
}

pub async fn stop_current_and_start(
    songbird: &Arc<Songbird>,
    music: &Arc<MusicManager>,
    resolver: &Arc<dyn TrackResolver>,
    guild_id: GuildId,
    old_handle: Option<TrackHandle>,
    next: Option<ResolvedTrack>,
    generation: u64,
) -> Result<()> {
    if let Some(handle) = old_handle {
        let _ = handle.stop();
    }

    if let Some(track) = next {
        start_playback(songbird, music, resolver, guild_id, generation, track)
            .await?;
    }

    Ok(())
}
