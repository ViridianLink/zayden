use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serenity::all::UserId;
use songbird::input::{HttpRequest, Input};
use songbird_reqwest::Client;
use zayden_app::config::{RadioStation, radio};

use super::http::stream_client;
use super::{Resolution, SourceQuery, TrackResolver};
use crate::error::{MusicError, Result};
use crate::track::{ResolvedTrack, TrackSource};

pub struct RadioResolver {
    http: Client,
    stations: Arc<[RadioStation]>,
}

impl RadioResolver {
    pub fn new(stations: Arc<[RadioStation]>) -> Result<Self> {
        Ok(Self { http: stream_client()?, stations })
    }

    #[must_use]
    pub fn stations(&self) -> &[RadioStation] {
        &self.stations
    }
}

#[must_use]
pub fn station_track(station: &RadioStation, requested_by: UserId) -> ResolvedTrack {
    ResolvedTrack {
        title: station.name.clone(),
        url: station.display_url().to_string(),
        source_id: station.id.clone(),
        source: TrackSource::Radio,
        duration: None,
        is_live: true,
        thumbnail_url: station.logo_url.clone(),
        requested_by,
    }
}

const HEALTHY_PLAY: Duration = Duration::from_secs(30);
const MAX_RETRIES: u8 = 3;

#[must_use]
pub const fn should_reconnect(played: Duration, retries: u8) -> bool {
    if played.as_secs() >= HEALTHY_PLAY.as_secs() {
        return true;
    }

    retries < MAX_RETRIES
}

#[must_use]
pub const fn next_retry_count(played: Duration, retries: u8) -> u8 {
    if played.as_secs() >= HEALTHY_PLAY.as_secs() {
        return 0;
    }

    retries.saturating_add(1)
}

#[async_trait]
impl TrackResolver for RadioResolver {
    async fn resolve(
        &self,
        _query: &SourceQuery,
        _requested_by: UserId,
    ) -> Result<Resolution> {
        Err(MusicError::UnsupportedSource)
    }

    async fn stream(&self, track: &ResolvedTrack) -> Result<Input> {
        let station = radio::find(&self.stations, &track.source_id)
            .ok_or_else(|| MusicError::UnknownStation(track.source_id.clone()))?;

        Ok(HttpRequest::new(self.http.clone(), station.stream_url.clone()).into())
    }
}
