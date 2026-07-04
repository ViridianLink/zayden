use std::time::Duration;

use serenity::all::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackSource {
    YouTube,
    Spotify,
}

#[derive(Debug, Clone)]
pub struct RequestedBy {
    pub user_id: UserId,
    pub display_name: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedTrack {
    pub title: String,
    pub url: String,
    pub source_id: String,
    pub source: TrackSource,
    pub duration: Option<Duration>,
    pub is_live: bool,
    pub thumbnail_url: Option<String>,
    pub requested_by: RequestedBy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LoopMode {
    #[default]
    Off,
    Track,
    Queue,
}
