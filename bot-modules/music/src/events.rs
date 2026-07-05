use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serenity::all::{ChannelId, GuildId, UserId};
use songbird::{Event, EventContext, EventHandler, Songbird};
use tracing::{error, warn};

use crate::manager::MusicManager;
use crate::resolve::TrackResolver;
use crate::voice;

pub struct TrackEndNotifier {
    pub guild_id: GuildId,
    pub generation: u64,
    pub music: Arc<MusicManager>,
    pub songbird: Arc<Songbird>,
    pub resolver: Arc<dyn TrackResolver>,
}

#[async_trait]
impl EventHandler for TrackEndNotifier {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let player = self.music.get(self.guild_id)?;

        let next = {
            let mut guard = player.lock().await;
            if guard.generation != self.generation {
                return None;
            }
            guard.advance_queue()
        };

        let next_track = next?;
        let next_generation = self.generation.wrapping_add(1);

        if let Err(e) = voice::start_playback(
            &self.songbird,
            &self.music,
            &self.resolver,
            self.guild_id,
            next_generation,
            next_track,
        )
        .await
        {
            error!(error = ?e, guild_id = %self.guild_id, "failed to start next track");
        }

        None
    }
}

pub struct InactivityCheck {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub bot_id: UserId,
    pub music: Arc<MusicManager>,
    pub songbird: Arc<Songbird>,
    pub auto_disconnect_secs: u64,
    pub stay_connected: bool,
}

#[async_trait]
impl EventHandler for InactivityCheck {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        if self.stay_connected {
            return None;
        }

        let player = self.music.get(self.guild_id)?;

        let should_disconnect = {
            let mut guard = player.lock().await;

            let alone = self.music.occupancy().non_bot_count(
                self.guild_id,
                self.channel_id,
                self.bot_id,
            ) == 0;
            let nothing_to_play = guard.current.is_none() && guard.queue.is_empty();

            if alone || nothing_to_play {
                let idle_since = *guard.idle_since.get_or_insert_with(Instant::now);
                idle_since.elapsed()
                    >= Duration::from_secs(self.auto_disconnect_secs)
            } else {
                guard.idle_since = None;
                false
            }
        };

        if should_disconnect {
            if let Err(e) = voice::leave(&self.songbird, self.guild_id).await {
                warn!(error = ?e, guild_id = %self.guild_id, "failed to auto-disconnect");
            }
            let _ = self.music.remove(self.guild_id);
            return Some(Event::Cancel);
        }

        None
    }
}
