use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serenity::all::UserId;
use songbird::input::{Input, YoutubeDl};
use songbird_reqwest::Client;
use tokio::process::Command;
use url::Url;

use super::{
    LazyTail,
    PlaylistOrigin,
    Resolution,
    SourceKind,
    SourceQuery,
    TrackResolver,
};
use crate::error::{MusicError, Result};
use crate::track::{RequestedBy, ResolvedTrack, TrackSource};

const PLAYLIST_CAP: u64 = 500;

pub const YT_DLP_PROGRAM: &str = "yt-dlp";

pub struct YouTubeResolver {
    http: Client,
}

impl YouTubeResolver {
    pub fn new() -> Result<Self> {
        Ok(Self { http: Client::new() })
    }

    async fn resolve_single(
        &self,
        url: &str,
        requested_by: UserId,
    ) -> Result<ResolvedTrack> {
        let output = run_yt_dlp(&["--no-playlist", url]).await?;
        output.into_track(requested_by).ok_or(MusicError::NoResults)
    }

    async fn resolve_search(
        &self,
        query: &str,
        requested_by: UserId,
    ) -> Result<ResolvedTrack> {
        let target = format!("ytsearch1:{query}");
        let output = run_yt_dlp(&["--flat-playlist", &target]).await?;
        output
            .entries
            .into_iter()
            .next()
            .and_then(|entry| entry.into_track(requested_by))
            .ok_or(MusicError::NoResults)
    }

    async fn resolve_playlist(
        &self,
        url: &str,
        requested_by: UserId,
    ) -> Result<Resolution> {
        let start = playlist_start_index(url);

        let head_output = run_yt_dlp(&[
            "--flat-playlist",
            "--playlist-items",
            &start.to_string(),
            url,
        ])
        .await?;
        let first = head_output
            .entries
            .into_iter()
            .next()
            .and_then(|entry| entry.into_track(requested_by))
            .ok_or(MusicError::NoResults)?;
        let head = vec![first];

        let url = url.to_string();
        let tail: LazyTail = Box::pin(async move {
            let items = format!(
                "{}:{}",
                start.saturating_add(1),
                start.saturating_add(PLAYLIST_CAP - 1)
            );
            let output =
                run_yt_dlp(&["--flat-playlist", "--playlist-items", &items, &url])
                    .await?;
            Ok(output
                .entries
                .into_iter()
                .filter_map(|entry| entry.into_track(requested_by))
                .collect())
        });

        Ok(Resolution {
            head,
            tail: Some(tail),
            origin: PlaylistOrigin::YouTubePlaylist,
        })
    }
}

#[async_trait]
impl TrackResolver for YouTubeResolver {
    async fn resolve(
        &self,
        query: &SourceQuery,
        requested_by: UserId,
    ) -> Result<Resolution> {
        let raw = query.raw.trim();

        match query.kind {
            SourceKind::YouTubeUrl if has_playlist(raw) => {
                self.resolve_playlist(raw, requested_by).await
            },
            SourceKind::YouTubeUrl => {
                let track = self.resolve_single(raw, requested_by).await?;
                Ok(Resolution {
                    head: vec![track],
                    tail: None,
                    origin: PlaylistOrigin::Single,
                })
            },
            SourceKind::Search => {
                let track = self.resolve_search(&query.raw, requested_by).await?;
                Ok(Resolution {
                    head: vec![track],
                    tail: None,
                    origin: PlaylistOrigin::Search,
                })
            },
            SourceKind::SpotifyUrl => Err(MusicError::UnsupportedSource),
        }
    }

    async fn stream(&self, track: &ResolvedTrack) -> Result<Input> {
        Ok(YoutubeDl::new(self.http.clone(), track.url.clone()).into())
    }
}

#[must_use]
pub fn has_playlist(raw: &str) -> bool {
    Url::parse(raw).is_ok_and(|url| {
        url.query_pairs().any(|(key, value)| key == "list" && !value.is_empty())
    })
}

#[must_use]
pub fn playlist_start_index(raw: &str) -> u64 {
    Url::parse(raw)
        .ok()
        .and_then(|url| {
            url.query_pairs()
                .find(|(key, _)| key == "index")
                .and_then(|(_, value)| value.parse::<u64>().ok())
        })
        .filter(|index| *index >= 1)
        .unwrap_or(1)
}

async fn run_yt_dlp(args: &[&str]) -> Result<YtDlpOutput> {
    let output = Command::new(YT_DLP_PROGRAM)
        .arg("--dump-single-json")
        .arg("--no-warnings")
        .args(args)
        .output()
        .await
        .map_err(|e| {
            MusicError::Resolve(format!("could not run `{YT_DLP_PROGRAM}`: {e}"))
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(MusicError::Resolve(format!(
            "`{YT_DLP_PROGRAM}` failed: {}",
            stderr.trim()
        )));
    }

    serde_json::from_slice(&output.stdout).map_err(|e| {
        MusicError::Resolve(format!("could not parse yt-dlp output: {e}"))
    })
}

pub async fn probe_yt_dlp() -> Result<String> {
    let output = Command::new(YT_DLP_PROGRAM)
        .arg("--version")
        .output()
        .await
        .map_err(|e| {
            MusicError::Internal(format!("could not run `{YT_DLP_PROGRAM}`: {e}"))
        })?;

    if !output.status.success() {
        return Err(MusicError::Internal(format!(
            "`{YT_DLP_PROGRAM} --version` exited with status {}",
            output.status
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[derive(Deserialize)]
struct YtDlpOutput {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    duration: Option<f64>,
    #[serde(default)]
    webpage_url: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    thumbnail: Option<String>,
    #[serde(default)]
    thumbnails: Vec<Thumbnail>,
    #[serde(default)]
    live_status: Option<String>,
    #[serde(default)]
    is_live: Option<bool>,
    #[serde(default)]
    entries: Vec<Self>,
}

#[derive(Deserialize)]
struct Thumbnail {
    url: String,
}

impl YtDlpOutput {
    fn into_track(self, requested_by: UserId) -> Option<ResolvedTrack> {
        let id = self.id?;

        let url = self
            .webpage_url
            .or(self.url)
            .unwrap_or_else(|| format!("https://www.youtube.com/watch?v={id}"));

        let is_live = self.is_live.unwrap_or(false)
            || self.live_status.as_deref() == Some("is_live");

        let duration = self
            .duration
            .filter(|secs| secs.is_finite() && *secs > 0.0)
            .map(Duration::from_secs_f64);

        let thumbnail_url = self
            .thumbnail
            .or_else(|| self.thumbnails.into_iter().next_back().map(|t| t.url));

        Some(ResolvedTrack {
            title: self.title.unwrap_or_else(|| id.clone()),
            url,
            source_id: id,
            source: TrackSource::YouTube,
            duration,
            is_live,
            thumbnail_url,
            requested_by: RequestedBy {
                user_id: requested_by,
                display_name: String::new(),
            },
        })
    }
}
