use std::collections::{HashSet, VecDeque};

use rand::seq::SliceRandom;
use serenity::all::UserId;

use crate::error::{MusicError, Result};
use crate::track::ResolvedTrack;

#[derive(Debug, Default)]
pub struct Queue {
    tracks: VecDeque<ResolvedTrack>,
}

impl Queue {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.tracks.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    #[must_use]
    pub fn get(&self, pos: usize) -> Option<&ResolvedTrack> {
        self.tracks.get(pos)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ResolvedTrack> {
        self.tracks.iter()
    }

    pub fn push(&mut self, track: ResolvedTrack) {
        self.tracks.push_back(track);
    }

    pub fn insert_top(&mut self, track: ResolvedTrack) {
        self.tracks.push_front(track);
    }

    pub fn pop_front(&mut self) -> Option<ResolvedTrack> {
        self.tracks.pop_front()
    }

    pub fn clear(&mut self) {
        self.tracks.clear();
    }

    pub fn remove(&mut self, pos: usize) -> Result<ResolvedTrack> {
        self.tracks.remove(pos).ok_or(MusicError::QueuePositionOutOfRange(pos))
    }

    pub fn skip_to(&mut self, pos: usize) -> Result<ResolvedTrack> {
        if pos >= self.tracks.len() {
            return Err(MusicError::QueuePositionOutOfRange(pos));
        }
        self.tracks.drain(..pos);
        self.tracks.pop_front().ok_or(MusicError::QueuePositionOutOfRange(pos))
    }

    pub fn move_song(&mut self, from: usize, to: usize) -> Result<()> {
        if to >= self.tracks.len() {
            return Err(MusicError::QueuePositionOutOfRange(to));
        }
        let track = self.remove(from)?;
        self.tracks.insert(to, track);
        Ok(())
    }

    pub fn dedupe(&mut self) -> usize {
        let mut seen = HashSet::new();
        let before = self.tracks.len();
        self.tracks.retain(|t| seen.insert(t.source_id.clone()));
        before - self.tracks.len()
    }

    pub fn shuffle(&mut self) {
        let mut rng = rand::rng();
        let mut vec: Vec<_> = self.tracks.drain(..).collect();
        vec.shuffle(&mut rng);
        self.tracks = vec.into();
    }

    pub fn cleanup(&mut self, voice_members: &HashSet<UserId>) -> usize {
        let before = self.tracks.len();
        self.tracks.retain(|t| voice_members.contains(&t.requested_by.user_id));
        before - self.tracks.len()
    }
}
