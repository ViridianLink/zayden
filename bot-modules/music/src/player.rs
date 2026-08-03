use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Instant;

use serenity::all::{GenericChannelId, UserId};
use songbird::tracks::TrackHandle;
use zayden_app::config::{MusicSettingsRow, RadioStation};
use zayden_core::as_u64;

use crate::queue::Queue;
use crate::track::{LoopMode, ResolvedTrack, TrackSource};

const MAX_HISTORY: usize = 20;

#[must_use]
pub fn volume_scalar(percent: u8) -> f32 {
    f32::from(percent.min(100)) / 100.0
}

pub struct NowPlaying {
    pub track: ResolvedTrack,
    pub handle: TrackHandle,
    pub started_at: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnnounceConfig {
    pub enabled: bool,
    pub channel: Option<GenericChannelId>,
}

impl AnnounceConfig {
    pub const DEFAULT: Self = Self { enabled: true, channel: None };
}

impl From<&MusicSettingsRow> for AnnounceConfig {
    fn from(settings: &MusicSettingsRow) -> Self {
        Self {
            enabled: settings.announce_now_playing,
            channel: settings
                .announce_channel_id
                .map(|id| GenericChannelId::new(as_u64(id))),
        }
    }
}

pub struct GuildPlayer {
    pub queue: Queue,
    pub current: Option<NowPlaying>,
    pub loop_mode: LoopMode,
    pub volume: u8,
    pub history: VecDeque<ResolvedTrack>,
    pub text_channel: GenericChannelId,
    pub skip_votes: HashSet<UserId>,
    pub generation: u64,
    pub idle_since: Option<Instant>,
    pub periodic_registered: bool,
    pub starting: bool,
    pub announce: AnnounceConfig,
    pub radio: Option<Arc<RadioStation>>,
    pub radio_retries: u8,
}

impl GuildPlayer {
    #[must_use]
    pub fn new(text_channel: GenericChannelId, default_volume: u8) -> Self {
        Self {
            queue: Queue::new(),
            current: None,
            loop_mode: LoopMode::Off,
            volume: default_volume,
            history: VecDeque::new(),
            text_channel,
            skip_votes: HashSet::new(),
            generation: 0,
            idle_since: None,
            periodic_registered: false,
            starting: false,
            announce: AnnounceConfig::DEFAULT,
            radio: None,
            radio_retries: 0,
        }
    }

    pub fn set_radio(&mut self, station: Arc<RadioStation>) {
        self.radio = Some(station);
        self.radio_retries = 0;
    }

    pub fn clear_radio(&mut self) -> bool {
        self.radio_retries = 0;
        self.radio.take().is_some()
    }

    pub const fn set_announce(&mut self, announce: AnnounceConfig) {
        self.announce = announce;
    }

    #[must_use]
    pub const fn announce_target(&self) -> Option<GenericChannelId> {
        if !self.announce.enabled {
            return None;
        }

        match self.announce.channel {
            Some(channel) => Some(channel),
            None => Some(self.text_channel),
        }
    }

    pub const fn try_begin_start(&mut self) -> bool {
        if self.current.is_some() || self.starting {
            return false;
        }
        self.starting = true;
        true
    }

    pub const fn finish_start(&mut self) {
        self.starting = false;
    }

    pub fn advance(&mut self) -> Option<ResolvedTrack> {
        self.generation = self.generation.wrapping_add(1);
        self.skip_votes.clear();

        let previous = self.current.take().map(|now_playing| now_playing.track);
        if let Some(track) = &previous
            && records_history(track.source)
        {
            self.history.push_back(track.clone());
            if self.history.len() > MAX_HISTORY {
                self.history.pop_front();
            }
        }
        previous
    }

    pub fn advance_queue(&mut self) -> Option<ResolvedTrack> {
        let finished = self.advance();
        let finished_is_radio =
            finished.as_ref().is_some_and(|t| t.source == TrackSource::Radio);

        match advance_action(self.loop_mode, finished_is_radio) {
            AdvanceAction::Replay => finished,
            AdvanceAction::Requeue => {
                if let Some(track) = finished {
                    self.queue.push(track);
                }
                self.queue.pop_front()
            },
            AdvanceAction::Drop => self.queue.pop_front(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvanceAction {
    Replay,
    Requeue,
    Drop,
}

#[must_use]
pub const fn advance_action(
    loop_mode: LoopMode,
    finished_is_radio: bool,
) -> AdvanceAction {
    if finished_is_radio {
        return AdvanceAction::Drop;
    }

    match loop_mode {
        LoopMode::Track => AdvanceAction::Replay,
        LoopMode::Queue => AdvanceAction::Requeue,
        LoopMode::Off => AdvanceAction::Drop,
    }
}

#[must_use]
pub const fn records_history(source: TrackSource) -> bool {
    !matches!(source, TrackSource::Radio)
}
