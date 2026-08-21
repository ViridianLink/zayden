use core::fmt::NumBuffer;
use std::collections::HashMap;
use std::process::Output;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::Deserialize;
use serenity::all::UserId;
use songbird::input::{HlsRequest, HttpRequest, Input};
use songbird_reqwest::Client;
use songbird_reqwest::header::{HeaderMap, HeaderName, HeaderValue, RANGE};
use tokio::process::Command;
use tracing::warn;
use url::Url;

use super::cookies::{CookieJar, cookie_warning};
use super::http::stream_client;
use super::{
    LazyTail,
    PlaylistOrigin,
    Resolution,
    SourceKind,
    SourceQuery,
    TrackResolver,
};
use crate::error::{MusicError, Result};
use crate::track::{ResolvedTrack, TrackSource};

const PLAYLIST_CAP: u64 = 500;

pub const YT_DLP_PROGRAM: &str = "yt-dlp";
pub const YT_DLP_TIMEOUT: Duration = Duration::from_secs(60);
pub const YT_DLP_PROBE_TIMEOUT: Duration = Duration::from_secs(10);
pub const YT_DLP_STREAM_TIMEOUT: Duration = Duration::from_secs(20);

pub const STREAM_FORMAT: &str = "ba[abr>0][vcodec=none]/ba/best";
pub const STREAM_CLIENTS: &[&str] =
    &["android_vr", "web_embedded", "mweb", "tv_simply"];
pub const AUTHED_STREAM_CLIENTS: &[&str] =
    &["web_embedded", "tv_downgraded", "web", "mweb"];
pub const COOKIE_UNSUPPORTED_CLIENTS: &[&str] =
    &["android", "android_vr", "ios", "visionos", "tv_simply"];

#[must_use]
pub const fn stream_clients(authenticated: bool) -> &'static [&'static str] {
    if authenticated { AUTHED_STREAM_CLIENTS } else { STREAM_CLIENTS }
}

pub struct YouTubeResolver {
    http: Client,
    cookies: Option<Arc<CookieJar>>,
}

impl YouTubeResolver {
    pub fn new() -> Result<Self> {
        Ok(Self { http: stream_client()?, cookies: None })
    }

    #[must_use]
    pub fn with_cookies(self, jar: Arc<CookieJar>) -> Self {
        Self { cookies: Some(jar), ..self }
    }

    fn jar(&self) -> Option<&CookieJar> {
        self.cookies.as_deref()
    }

    async fn resolve_single(
        &self,
        url: &str,
        requested_by: UserId,
    ) -> Result<ResolvedTrack> {
        let output = run_yt_dlp(self.jar(), &["--no-playlist", url]).await?;
        output.into_track(requested_by).ok_or(MusicError::NoResults)
    }

    async fn resolve_search(
        &self,
        query: &str,
        requested_by: UserId,
    ) -> Result<ResolvedTrack> {
        let target = format!("ytsearch1:{query}");
        let output = run_yt_dlp(self.jar(), &["--flat-playlist", &target]).await?;
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

        let mut buf = NumBuffer::<u64>::new();
        let head_output = run_yt_dlp(self.jar(), &[
            "--flat-playlist",
            "--playlist-items",
            start.format_into(&mut buf),
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
        let cookies = self.cookies.clone();
        let tail: LazyTail = Box::pin(async move {
            let items = format!(
                "{}:{}",
                start.saturating_add(1),
                start.saturating_add(PLAYLIST_CAP - 1)
            );
            let output = run_yt_dlp(cookies.as_deref(), &[
                "--flat-playlist",
                "--playlist-items",
                &items,
                &url,
            ])
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

    async fn prepare_stream(&self, url: &str, client: &str) -> Result<Input> {
        let player_client = format!("youtube:player_client={client}");
        let output = run_yt_dlp_within(self.jar(), YT_DLP_STREAM_TIMEOUT, &[
            "--format",
            STREAM_FORMAT,
            "--no-playlist",
            "--extractor-args",
            &player_client,
            url,
        ])
        .await?;

        let format = output.into_stream_format().ok_or(MusicError::NoResults)?;

        if format.is_hls() {
            return Ok(HlsRequest::new_with_headers(
                self.http.clone(),
                format.url,
                format.headers,
            )
            .into());
        }

        probe_stream(&self.http, &format).await?;

        Ok(HttpRequest {
            client: self.http.clone(),
            request: format.url,
            headers: format.headers,
            content_length: format.filesize,
        }
        .into())
    }
}

pub async fn probe_stream(http: &Client, format: &StreamFormat) -> Result<()> {
    let mut request = http.get(&format.url).headers(format.headers.clone());

    if let Some(range) = format.range_header() {
        request = request.header(RANGE, range);
    }

    let response = request.send().await.map_err(|e| {
        MusicError::Resolve(format!("could not reach the audio host: {e}"))
    })?;

    if !response.status().is_success() {
        return Err(MusicError::Resolve(format!(
            "the audio host rejected the stream URL: {}",
            response.status()
        )));
    }

    Ok(())
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
        let mut last = None;

        for client in stream_clients(self.cookies.is_some()) {
            match self.prepare_stream(&track.url, client).await {
                Ok(input) => return Ok(input),
                Err(e) => {
                    warn!(
                        player_client = client,
                        url = %track.url,
                        "no playable stream from this YouTube client: {e}"
                    );
                    last = Some(e);
                },
            }
        }

        Err(last.unwrap_or(MusicError::NoResults))
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

pub async fn run_with_timeout(
    program: &str,
    args: &[&str],
    budget: Duration,
) -> std::result::Result<Output, String> {
    let run = Command::new(program).args(args).kill_on_drop(true).output();

    match tokio::time::timeout(budget, run).await {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(format!("could not run `{program}`: {e}")),
        Err(_) => Err(format!(
            "`{program}` did not finish within {}s and was killed",
            budget.as_secs()
        )),
    }
}

async fn run_yt_dlp(
    cookies: Option<&CookieJar>,
    args: &[&str],
) -> Result<YtDlpOutput> {
    run_yt_dlp_within(cookies, YT_DLP_TIMEOUT, args).await
}

async fn run_yt_dlp_within(
    cookies: Option<&CookieJar>,
    budget: Duration,
    args: &[&str],
) -> Result<YtDlpOutput> {
    let lease = match cookies {
        Some(jar) => Some(jar.lease().await?),
        None => None,
    };

    let mut full = vec!["--dump-single-json"];
    match &lease {
        Some(lease) => full.extend_from_slice(&["--cookies", lease.arg()]),
        None => full.push("--no-warnings"),
    }
    full.extend_from_slice(args);

    let output = run_with_timeout(YT_DLP_PROGRAM, &full, budget)
        .await
        .map_err(MusicError::Resolve)?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    if lease.is_some()
        && let Some(warning) = cookie_warning(&stderr)
    {
        warn!(
            "the configured YouTube cookie file is no longer signing in; \
             re-export it from a logged-in browser session: {warning}"
        );
    }

    if !output.status.success() {
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
    let output =
        run_with_timeout(YT_DLP_PROGRAM, &["--version"], YT_DLP_PROBE_TIMEOUT)
            .await
            .map_err(MusicError::Internal)?;

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
    #[serde(default)]
    http_headers: HashMap<String, String>,
    #[serde(default)]
    filesize: Option<u64>,
    #[serde(default)]
    protocol: Option<String>,
}

#[derive(Deserialize)]
struct Thumbnail {
    url: String,
}

pub struct StreamFormat {
    pub url: String,
    pub headers: HeaderMap,
    pub filesize: Option<u64>,
    pub protocol: Option<String>,
}

impl StreamFormat {
    #[must_use]
    pub fn is_hls(&self) -> bool {
        self.protocol.as_deref().is_some_and(|p| p.starts_with("m3u8"))
    }

    #[must_use]
    pub fn range_header(&self) -> Option<String> {
        self.filesize.map(|max| format!("bytes=0-{}", max.saturating_sub(1)))
    }
}

impl YtDlpOutput {
    fn into_stream_format(self) -> Option<StreamFormat> {
        let url = self.url?;

        let headers = self
            .http_headers
            .iter()
            .filter_map(|(name, value)| {
                Some((
                    HeaderName::from_bytes(name.as_bytes()).ok()?,
                    HeaderValue::from_str(value).ok()?,
                ))
            })
            .collect();

        Some(StreamFormat {
            url,
            headers,
            filesize: self.filesize,
            protocol: self.protocol,
        })
    }

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
            requested_by,
        })
    }
}
