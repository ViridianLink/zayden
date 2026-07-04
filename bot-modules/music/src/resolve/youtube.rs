use std::time::Duration;

use async_trait::async_trait;
use rusty_ytdl::search::{
    Playlist,
    PlaylistSearchOptions,
    SearchResult,
    Video as SearchVideo,
    YouTube as YouTubeSearch,
};
use rusty_ytdl::{
    VideoDetails,
    VideoOptions,
    VideoQuality,
    VideoSearchOptions,
    choose_format,
};
use serenity::all::UserId;
use songbird::input::{HttpRequest, Input};
use songbird_reqwest::Client;

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

pub struct YouTubeResolver {
    search: YouTubeSearch,
    http: Client,
}

impl YouTubeResolver {
    pub fn new() -> Result<Self> {
        Ok(Self {
            search: YouTubeSearch::new()
                .map_err(|e| MusicError::Resolve(e.to_string()))?,
            http: Client::new(),
        })
    }

    fn video_options() -> VideoOptions {
        VideoOptions {
            quality: VideoQuality::HighestAudio,
            filter: VideoSearchOptions::Audio,
            ..Default::default()
        }
    }

    async fn resolve_single(
        &self,
        url_or_id: &str,
        requested_by: UserId,
    ) -> Result<ResolvedTrack> {
        let video =
            rusty_ytdl::Video::new_with_options(url_or_id, Self::video_options())
                .map_err(|e| MusicError::Resolve(e.to_string()))?;
        let info = video
            .get_info()
            .await
            .map_err(|e| MusicError::Resolve(e.to_string()))?;
        Ok(from_video_details(&info.video_details, requested_by))
    }

    async fn resolve_search(
        &self,
        query: &str,
        requested_by: UserId,
    ) -> Result<ResolvedTrack> {
        let result = self
            .search
            .search_one(query, None)
            .await
            .map_err(|e| MusicError::Resolve(e.to_string()))?
            .ok_or(MusicError::NoResults)?;

        match result {
            SearchResult::Video(video) => {
                Ok(from_search_video(&video, requested_by))
            },
            SearchResult::Playlist(_) | SearchResult::Channel(_) => {
                Err(MusicError::NoResults)
            },
        }
    }

    async fn resolve_playlist(
        &self,
        url: &str,
        requested_by: UserId,
    ) -> Result<Resolution> {
        let head_playlist = Playlist::get(
            url,
            Some(&PlaylistSearchOptions { limit: 1, ..Default::default() }),
        )
        .await
        .map_err(|e| MusicError::Resolve(e.to_string()))?;

        let first = head_playlist.videos.first().ok_or(MusicError::NoResults)?;
        let head = vec![from_search_video(first, requested_by)];

        let url = url.to_string();
        let tail: LazyTail = Box::pin(async move {
            let playlist = Playlist::get(
                &url,
                Some(&PlaylistSearchOptions {
                    limit: PLAYLIST_CAP,
                    ..Default::default()
                }),
            )
            .await
            .map_err(|e| MusicError::Resolve(e.to_string()))?;

            Ok(playlist
                .videos
                .into_iter()
                .skip(1)
                .map(|video| from_search_video(&video, requested_by))
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
        match query.kind {
            SourceKind::YouTubeUrl if Playlist::is_playlist(query.raw.trim()) => {
                self.resolve_playlist(query.raw.trim(), requested_by).await
            },
            SourceKind::YouTubeUrl => {
                let track =
                    self.resolve_single(query.raw.trim(), requested_by).await?;
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
        let video = rusty_ytdl::Video::new_with_options(
            &track.source_id,
            Self::video_options(),
        )
        .map_err(|e| MusicError::Resolve(e.to_string()))?;
        let info = video
            .get_info()
            .await
            .map_err(|e| MusicError::Resolve(e.to_string()))?;
        let format = choose_format(&info.formats, &Self::video_options())
            .map_err(|e| MusicError::Resolve(e.to_string()))?;

        Ok(HttpRequest::new(self.http.clone(), format.url).into())
    }
}

fn from_video_details(
    details: &VideoDetails,
    requested_by: UserId,
) -> ResolvedTrack {
    let duration = details.length_seconds.parse().ok().map(Duration::from_secs);

    ResolvedTrack {
        title: details.title.clone(),
        url: details.video_url.clone(),
        source_id: details.video_id.clone(),
        source: TrackSource::YouTube,
        duration,
        is_live: details.is_live_content,
        thumbnail_url: details.thumbnails.last().map(|t| t.url.clone()),
        requested_by: RequestedBy {
            user_id: requested_by,
            display_name: String::new(),
        },
    }
}

fn from_search_video(video: &SearchVideo, requested_by: UserId) -> ResolvedTrack {
    ResolvedTrack {
        title: video.title.clone(),
        url: video.url.clone(),
        source_id: video.id.clone(),
        source: TrackSource::YouTube,
        duration: (video.duration > 0).then(|| Duration::from_secs(video.duration)),
        is_live: video.duration == 0,
        thumbnail_url: video.thumbnails.last().map(|t| t.url.clone()),
        requested_by: RequestedBy {
            user_id: requested_by,
            display_name: String::new(),
        },
    }
}
