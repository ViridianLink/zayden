use std::time::Duration as StdDuration;

use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use rspotify::clients::BaseClient;
use rspotify::model::{
    AlbumId,
    FullAlbum,
    FullTrack,
    PlayableItem,
    PlaylistId,
    PlaylistItem,
    SimplifiedArtist,
    SimplifiedTrack,
    TrackId,
};
use rspotify::{ClientCredsSpotify, Credentials};
use serenity::all::UserId;
use songbird::input::Input;
use url::Url;

use super::{
    LazyTail,
    PlaylistOrigin,
    Resolution,
    SourceKind,
    SourceQuery,
    TrackResolver,
    YouTubeResolver,
};
use crate::error::{MusicError, Result};
use crate::track::{RequestedBy, ResolvedTrack, TrackSource};

const PLAYLIST_CAP: usize = 500;

#[derive(Debug, PartialEq, Eq)]
pub enum SpotifyKind {
    Track,
    Album,
    Playlist,
}

pub struct SpotifyResolver {
    client: ClientCredsSpotify,
}

impl SpotifyResolver {
    pub async fn new(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Result<Self> {
        let creds = Credentials::new(&client_id.into(), &client_secret.into());
        let client = ClientCredsSpotify::new(creds);
        client
            .request_token()
            .await
            .map_err(|e| MusicError::Resolve(e.to_string()))?;

        Ok(Self { client })
    }

    async fn resolve_track(
        &self,
        id: &str,
        requested_by: UserId,
    ) -> Result<ResolvedTrack> {
        let track_id = TrackId::from_id(id.to_string())
            .map_err(|e| MusicError::Resolve(e.to_string()))?;
        let track = self
            .client
            .track(track_id, None)
            .await
            .map_err(|e| MusicError::Resolve(e.to_string()))?;

        Ok(from_full_track(&track, requested_by))
    }

    async fn resolve_album(
        &self,
        id: &str,
        requested_by: UserId,
    ) -> Result<Resolution> {
        let album_id = AlbumId::from_id(id.to_string())
            .map_err(|e| MusicError::Resolve(e.to_string()))?;
        let album: FullAlbum = self
            .client
            .album(album_id, None)
            .await
            .map_err(|e| MusicError::Resolve(e.to_string()))?;

        let mut tracks = album.tracks.items.into_iter();
        let first = tracks.next().ok_or(MusicError::NoResults)?;
        let head = vec![from_simplified_track(&first, requested_by)];

        let rest: Vec<ResolvedTrack> = tracks
            .take(PLAYLIST_CAP - 1)
            .map(|track| from_simplified_track(&track, requested_by))
            .collect();
        let tail: LazyTail = Box::pin(async move { Ok(rest) });

        Ok(Resolution {
            head,
            tail: Some(tail),
            origin: PlaylistOrigin::SpotifyPlaylist,
        })
    }

    async fn resolve_playlist(
        &self,
        id: &str,
        requested_by: UserId,
    ) -> Result<Resolution> {
        let head_id = PlaylistId::from_id(id.to_string())
            .map_err(|e| MusicError::Resolve(e.to_string()))?;

        let head_items: Vec<PlaylistItem> = self
            .client
            .playlist_items(head_id, None, None)
            .take(1)
            .try_collect()
            .await
            .map_err(|e| MusicError::Resolve(e.to_string()))?;

        let first = head_items.into_iter().next().ok_or(MusicError::NoResults)?;
        let head = vec![from_playable_item(&first, requested_by)?];

        let client = self.client.clone();
        let id = id.to_string();
        let tail: LazyTail = Box::pin(async move {
            let playlist_id = PlaylistId::from_id(id)
                .map_err(|e| MusicError::Resolve(e.to_string()))?;

            let items: Vec<PlaylistItem> = client
                .playlist_items(playlist_id, None, None)
                .skip(1)
                .take(PLAYLIST_CAP - 1)
                .try_collect()
                .await
                .map_err(|e| MusicError::Resolve(e.to_string()))?;

            Ok(items
                .iter()
                .filter_map(|item| from_playable_item(item, requested_by).ok())
                .collect())
        });

        Ok(Resolution {
            head,
            tail: Some(tail),
            origin: PlaylistOrigin::SpotifyPlaylist,
        })
    }
}

#[async_trait]
impl TrackResolver for SpotifyResolver {
    async fn resolve(
        &self,
        query: &SourceQuery,
        requested_by: UserId,
    ) -> Result<Resolution> {
        let (kind, id) = parse_spotify_url(&query.raw)?;

        match kind {
            SpotifyKind::Track => {
                let track = self.resolve_track(&id, requested_by).await?;
                Ok(Resolution {
                    head: vec![track],
                    tail: None,
                    origin: PlaylistOrigin::Single,
                })
            },
            SpotifyKind::Album => self.resolve_album(&id, requested_by).await,
            SpotifyKind::Playlist => self.resolve_playlist(&id, requested_by).await,
        }
    }

    async fn stream(&self, _track: &ResolvedTrack) -> Result<Input> {
        Err(MusicError::UnsupportedSource)
    }
}

pub struct CompositeResolver {
    youtube: YouTubeResolver,
    spotify: Option<SpotifyResolver>,
}

impl CompositeResolver {
    #[must_use]
    pub const fn new(
        youtube: YouTubeResolver,
        spotify: Option<SpotifyResolver>,
    ) -> Self {
        Self { youtube, spotify }
    }
}

#[async_trait]
impl TrackResolver for CompositeResolver {
    async fn resolve(
        &self,
        query: &SourceQuery,
        requested_by: UserId,
    ) -> Result<Resolution> {
        match query.kind {
            SourceKind::SpotifyUrl => match &self.spotify {
                Some(spotify) => spotify.resolve(query, requested_by).await,
                None => Err(MusicError::SpotifyDisabled),
            },
            SourceKind::YouTubeUrl | SourceKind::Search => {
                self.youtube.resolve(query, requested_by).await
            },
        }
    }

    async fn stream(&self, track: &ResolvedTrack) -> Result<Input> {
        match track.source {
            TrackSource::YouTube => self.youtube.stream(track).await,
            TrackSource::Spotify => {
                let query = SourceQuery::new(track.title.clone());
                let resolution =
                    self.youtube.resolve(&query, track.requested_by.user_id).await?;
                let yt_track = resolution
                    .head
                    .into_iter()
                    .next()
                    .ok_or(MusicError::NoResults)?;
                self.youtube.stream(&yt_track).await
            },
        }
    }
}

pub fn parse_spotify_url(raw: &str) -> Result<(SpotifyKind, String)> {
    let Ok(url) = Url::parse(raw.trim()) else {
        return Err(MusicError::UnsupportedSource);
    };
    let mut segments = url.path_segments().ok_or(MusicError::UnsupportedSource)?;
    let kind = segments.next().ok_or(MusicError::UnsupportedSource)?;
    let id = segments.next().ok_or(MusicError::UnsupportedSource)?.to_string();

    let kind = match kind {
        "track" => SpotifyKind::Track,
        "album" => SpotifyKind::Album,
        "playlist" => SpotifyKind::Playlist,
        _ => return Err(MusicError::UnsupportedSource),
    };

    Ok((kind, id))
}

fn search_query(name: &str, artists: &[SimplifiedArtist]) -> String {
    let artist = artists.first().map(|a| a.name.as_str()).unwrap_or_default();
    format!("{artist} - {name}")
}

fn from_full_track(track: &FullTrack, requested_by: UserId) -> ResolvedTrack {
    let source_id = track.id.as_ref().map(ToString::to_string).unwrap_or_default();

    ResolvedTrack {
        title: search_query(&track.name, &track.artists),
        url: format!("https://open.spotify.com/track/{source_id}"),
        source_id,
        source: TrackSource::Spotify,
        duration: track.duration.to_std().ok().map(|d: StdDuration| d),
        is_live: false,
        thumbnail_url: track.album.images.first().map(|i| i.url.clone()),
        requested_by: RequestedBy {
            user_id: requested_by,
            display_name: String::new(),
        },
    }
}

fn from_simplified_track(
    track: &SimplifiedTrack,
    requested_by: UserId,
) -> ResolvedTrack {
    let source_id = track.id.as_ref().map(ToString::to_string).unwrap_or_default();

    ResolvedTrack {
        title: search_query(&track.name, &track.artists),
        url: format!("https://open.spotify.com/track/{source_id}"),
        source_id,
        source: TrackSource::Spotify,
        duration: track.duration.to_std().ok(),
        is_live: false,
        thumbnail_url: track
            .album
            .as_ref()
            .and_then(|a| a.images.first())
            .map(|i| i.url.clone()),
        requested_by: RequestedBy {
            user_id: requested_by,
            display_name: String::new(),
        },
    }
}

fn from_playable_item(
    item: &PlaylistItem,
    requested_by: UserId,
) -> Result<ResolvedTrack> {
    match &item.item {
        Some(PlayableItem::Track(track)) => Ok(from_full_track(track, requested_by)),
        _ => Err(MusicError::NoResults),
    }
}
