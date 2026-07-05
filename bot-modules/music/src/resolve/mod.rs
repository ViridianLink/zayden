use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use serenity::all::UserId;
use songbird::input::Input;
use url::Url;

use crate::error::Result;
use crate::track::ResolvedTrack;

pub mod spotify;
pub mod youtube;

pub use spotify::{
    CompositeResolver,
    SpotifyKind,
    SpotifyResolver,
    parse_spotify_url,
};
pub use youtube::{YouTubeResolver, probe_yt_dlp};

#[async_trait]
pub trait TrackResolver: Send + Sync {
    async fn resolve(
        &self,
        query: &SourceQuery,
        requested_by: UserId,
    ) -> Result<Resolution>;

    async fn stream(&self, track: &ResolvedTrack) -> Result<Input>;
}

pub type LazyTail = Pin<Box<dyn Future<Output = Result<Vec<ResolvedTrack>>> + Send>>;

pub struct Resolution {
    pub head: Vec<ResolvedTrack>,
    pub tail: Option<LazyTail>,
    pub origin: PlaylistOrigin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaylistOrigin {
    Single,
    YouTubePlaylist,
    SpotifyPlaylist,
    Search,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    YouTubeUrl,
    SpotifyUrl,
    Search,
}

impl SourceKind {
    #[must_use]
    pub fn classify(query: &str) -> Self {
        let Ok(url) = Url::parse(query.trim()) else {
            return Self::Search;
        };

        match url.host_str() {
            Some(host) if is_youtube_host(host) => Self::YouTubeUrl,
            Some(host) if is_spotify_host(host) => Self::SpotifyUrl,
            _ => Self::Search,
        }
    }
}

fn is_youtube_host(host: &str) -> bool {
    matches!(
        host,
        "youtube.com"
            | "www.youtube.com"
            | "m.youtube.com"
            | "youtu.be"
            | "music.youtube.com"
    )
}

fn is_spotify_host(host: &str) -> bool {
    host == "open.spotify.com"
}

pub struct SourceQuery {
    pub raw: String,
    pub kind: SourceKind,
}

impl SourceQuery {
    #[must_use]
    pub fn new(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let kind = SourceKind::classify(&raw);
        Self { raw, kind }
    }
}
