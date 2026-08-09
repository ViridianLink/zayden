use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use serenity::all::UserId;
use songbird::input::Input;
use url::Url;

use crate::error::Result;
use crate::track::ResolvedTrack;

pub mod http;
pub mod radio;
pub mod spotify;
pub mod spotify_embed;
pub mod youtube;

pub use http::{STREAM_READ_TIMEOUT, stream_client, stream_client_with};
pub use radio::{RadioResolver, next_retry_count, should_reconnect, station_track};
pub use spotify::{
    CompositeResolver,
    SpotifyKind,
    SpotifyResolver,
    parse_spotify_url,
};
pub use spotify_embed::{
    EMBED_TRACK_LIMIT,
    EmbedPlaylist,
    EmbedTrack,
    embed_resolution,
    embed_url,
    fetch_embed_playlist,
    parse_embed_playlist,
};
pub use youtube::{
    YT_DLP_PROBE_TIMEOUT,
    YT_DLP_TIMEOUT,
    YouTubeResolver,
    has_playlist,
    playlist_start_index,
    probe_yt_dlp,
    run_with_timeout,
};

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
