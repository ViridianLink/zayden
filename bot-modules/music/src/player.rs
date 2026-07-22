use std::collections::{HashSet, VecDeque};
use std::time::Instant;

use serenity::all::{GenericChannelId, UserId};
use songbird::tracks::TrackHandle;

use crate::queue::Queue;
use crate::track::{LoopMode, ResolvedTrack};

const MAX_HISTORY: usize = 20;

pub struct NowPlaying {
    pub track: ResolvedTrack,
    pub handle: TrackHandle,
    pub started_at: Instant,
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
        if let Some(track) = &previous {
            self.history.push_back(track.clone());
            if self.history.len() > MAX_HISTORY {
                self.history.pop_front();
            }
        }
        previous
    }

    pub fn advance_queue(&mut self) -> Option<ResolvedTrack> {
        let finished = self.advance();
        match self.loop_mode {
            LoopMode::Track => finished,
            LoopMode::Queue => {
                if let Some(track) = finished {
                    self.queue.push(track);
                }
                self.queue.pop_front()
            },
            LoopMode::Off => self.queue.pop_front(),
        }
    }
}
