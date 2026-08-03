use std::sync::Arc;
use std::time::{Duration, Instant};

use async_trait::async_trait;
use serenity::all::{ChannelId, CreateMessage, GuildId, UserId};
use songbird::tracks::PlayMode;
use songbird::{Event, EventContext, EventHandler, Songbird};
use tracing::{error, warn};
use zayden_app::entitlement::{EntitlementScope, EntitlementService, Tier};

use crate::manager::MusicManager;
use crate::resolve::{next_retry_count, should_reconnect, station_track};
use crate::voice::Playback;
use crate::{embeds, voice};

pub struct TrackErrorNotifier {
    pub guild_id: GuildId,
    pub title: String,
}

#[async_trait]
impl EventHandler for TrackErrorNotifier {
    async fn act(&self, ctx: &EventContext<'_>) -> Option<Event> {
        if let EventContext::Track(states) = ctx {
            for (state, _) in *states {
                if let PlayMode::Errored(err) = &state.playing {
                    error!(
                        guild_id = %self.guild_id,
                        title = %self.title,
                        "track playback failed: {err}"
                    );
                }
            }
        }

        None
    }
}

pub struct TrackEndNotifier {
    pub guild_id: GuildId,
    pub generation: u64,
    pub playback: Playback,
}

#[async_trait]
impl EventHandler for TrackEndNotifier {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        let player = self.playback.music.get(self.guild_id)?;

        let (next, announce_to) = {
            let mut guard = player.lock().await;
            if guard.generation != self.generation {
                return None;
            }

            let reconnect = match (guard.radio.clone(), guard.current.as_ref()) {
                (Some(station), Some(now)) => {
                    let played = now.started_at.elapsed();
                    let retries = guard.radio_retries;

                    should_reconnect(played, retries).then(|| {
                        let track = station_track(&station, now.track.requested_by);
                        (station, track, next_retry_count(played, retries))
                    })
                },
                _ => None,
            };

            match reconnect {
                Some((station, track, retries)) => {
                    guard.radio_retries = retries;
                    guard.advance();
                    let generation = guard.generation;
                    drop(guard);

                    if let Err(e) = voice::start_playback(
                        &self.playback,
                        self.guild_id,
                        generation,
                        track,
                    )
                    .await
                    {
                        error!(
                            error = ?e,
                            guild_id = %self.guild_id,
                            station = %station.id,
                            "failed to reconnect to radio station"
                        );
                    }

                    return None;
                },
                None => {
                    if let Some(station) = guard.radio.as_ref() {
                        warn!(
                            guild_id = %self.guild_id,
                            station = %station.id,
                            "radio station kept dropping; leaving radio mode"
                        );
                        guard.clear_radio();
                    }
                },
            }

            (guard.advance_queue(), guard.announce_target())
        };

        let next_track = next?;
        let next_generation = self.generation.wrapping_add(1);
        let announcement = embeds::track_announcement_embed(&next_track);

        if let Err(e) = voice::start_playback(
            &self.playback,
            self.guild_id,
            next_generation,
            next_track,
        )
        .await
        {
            error!(error = ?e, guild_id = %self.guild_id, "failed to start next track");
            return None;
        }

        if let Some(channel) = announce_to
            && let Err(e) = channel
                .send_message(
                    &self.playback.http,
                    CreateMessage::new().embed(announcement),
                )
                .await
        {
            warn!(error = ?e, guild_id = %self.guild_id, "failed to announce next track");
        }

        None
    }
}

pub struct InactivityCheck {
    pub guild_id: GuildId,
    pub channel_id: ChannelId,
    pub bot_id: UserId,
    pub user_id: UserId,
    pub music: Arc<MusicManager>,
    pub songbird: Arc<Songbird>,
    pub entitlements: Arc<EntitlementService>,
    pub auto_disconnect_secs: u64,
    pub stay_connected: bool,
}

#[async_trait]
impl EventHandler for InactivityCheck {
    async fn act(&self, _ctx: &EventContext<'_>) -> Option<Event> {
        if self.stay_connected {
            let scope = EntitlementScope::UserInGuild(
                self.user_id.get(),
                self.guild_id.get(),
            );

            if self.entitlements.allows(scope, Tier::Pro).await {
                return None;
            }
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
