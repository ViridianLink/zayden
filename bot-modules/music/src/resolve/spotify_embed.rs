use std::time::Duration;

use serde::Deserialize;
use serenity::all::UserId;
use songbird_reqwest::Client;

use super::{LazyTail, PlaylistOrigin, Resolution};
use crate::error::{MusicError, Result};
use crate::track::{ResolvedTrack, TrackSource};

pub const EMBED_TRACK_LIMIT: usize = 100;

const NEXT_DATA_MARKER: &str = r#"<script id="__NEXT_DATA__""#;
const TRACK_URI_PREFIX: &str = "spotify:track:";
const EMBED_USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) \
     Chrome/140.0.0.0 Safari/537.36";

#[derive(Debug, Clone)]
pub struct EmbedTrack {
    pub id: String,
    pub title: String,
    pub artists: String,
    pub duration: Option<Duration>,
}

#[derive(Debug, Clone)]
pub struct EmbedPlaylist {
    pub name: String,
    pub cover_url: Option<String>,
    pub tracks: Vec<EmbedTrack>,
}

#[must_use]
pub fn embed_url(playlist_id: &str) -> String {
    format!("https://open.spotify.com/embed/playlist/{playlist_id}")
}

pub async fn fetch_embed_playlist(
    client: &Client,
    playlist_id: &str,
) -> Result<EmbedPlaylist> {
    let response = client
        .get(embed_url(playlist_id))
        .header(songbird_reqwest::header::USER_AGENT, EMBED_USER_AGENT)
        .send()
        .await
        .map_err(|e| {
            MusicError::Resolve(format!(
                "could not reach the Spotify embed page: {e}"
            ))
        })?
        .error_for_status()
        .map_err(|e| {
            MusicError::Resolve(format!("Spotify rejected the embed request: {e}"))
        })?;

    let html = response.text().await.map_err(|e| {
        MusicError::Resolve(format!("could not read the Spotify embed page: {e}"))
    })?;

    parse_embed_playlist(&html)
}

pub fn parse_embed_playlist(html: &str) -> Result<EmbedPlaylist> {
    let json = extract_next_data(html).ok_or_else(|| {
        MusicError::Resolve(
            "the Spotify embed page carried no playlist data".to_owned(),
        )
    })?;

    let data: NextData = serde_json::from_str(json.trim()).map_err(|e| {
        MusicError::Resolve(format!("could not read the Spotify embed data: {e}"))
    })?;
    let entity = data.props.page_props.state.data.entity;

    let tracks: Vec<EmbedTrack> =
        entity.track_list.iter().filter_map(embed_track).collect();
    if tracks.is_empty() {
        return Err(MusicError::NoResults);
    }

    let cover_url = entity
        .cover_art
        .and_then(|art| art.sources.into_iter().next())
        .map(|source| source.url);

    Ok(EmbedPlaylist { name: entity.name, cover_url, tracks })
}

#[must_use]
pub fn embed_resolution(
    playlist: EmbedPlaylist,
    requested_by: UserId,
) -> Resolution {
    let cover_url = playlist.cover_url;

    let mut tracks: Vec<ResolvedTrack> = playlist
        .tracks
        .into_iter()
        .take(EMBED_TRACK_LIMIT)
        .map(|track| resolved_track(track, cover_url.as_deref(), requested_by))
        .collect();

    let rest = tracks.split_off(tracks.len().min(1));
    let tail: LazyTail = Box::pin(async move { Ok(rest) });

    Resolution {
        head: tracks,
        tail: Some(tail),
        origin: PlaylistOrigin::SpotifyPlaylist,
    }
}

fn resolved_track(
    track: EmbedTrack,
    cover_url: Option<&str>,
    requested_by: UserId,
) -> ResolvedTrack {
    let EmbedTrack { id, title, artists, duration } = track;

    let title = match lead_artist(&artists) {
        Some(artist) => format!("{artist} - {title}"),
        None => title,
    };

    ResolvedTrack {
        url: format!("https://open.spotify.com/track/{id}"),
        title,
        source_id: id,
        source: TrackSource::Spotify,
        duration,
        is_live: false,
        thumbnail_url: cover_url.map(ToOwned::to_owned),
        requested_by,
    }
}

fn lead_artist(artists: &str) -> Option<&str> {
    let lead = artists.split(',').next()?.trim();

    (!lead.is_empty()).then_some(lead)
}

fn extract_next_data(html: &str) -> Option<&str> {
    let (_, rest) = html.split_once(NEXT_DATA_MARKER)?;
    let (_, rest) = rest.split_once('>')?;
    let (json, _) = rest.split_once("</script>")?;

    Some(json)
}

fn embed_track(raw: &RawTrack) -> Option<EmbedTrack> {
    let id = raw.uri.strip_prefix(TRACK_URI_PREFIX)?;

    Some(EmbedTrack {
        id: id.to_owned(),
        title: raw.title.clone(),
        artists: normalise_spaces(&raw.subtitle),
        duration: raw.duration.filter(|ms| *ms > 0).map(Duration::from_millis),
    })
}

fn normalise_spaces(text: &str) -> String {
    text.replace('\u{a0}', " ")
}

#[derive(Deserialize)]
struct NextData {
    props: Props,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Props {
    page_props: PageProps,
}

#[derive(Deserialize)]
struct PageProps {
    state: State,
}

#[derive(Deserialize)]
struct State {
    data: StateData,
}

#[derive(Deserialize)]
struct StateData {
    entity: Entity,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entity {
    #[serde(default)]
    name: String,
    #[serde(default)]
    cover_art: Option<CoverArt>,
    #[serde(default)]
    track_list: Vec<RawTrack>,
}

#[derive(Deserialize)]
struct CoverArt {
    #[serde(default)]
    sources: Vec<CoverSource>,
}

#[derive(Deserialize)]
struct CoverSource {
    url: String,
}

#[derive(Deserialize)]
struct RawTrack {
    uri: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    subtitle: String,
    #[serde(default)]
    duration: Option<u64>,
}
