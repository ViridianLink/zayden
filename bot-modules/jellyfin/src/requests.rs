use std::sync::Arc;

use serde_json::{Value, json};
use tracing::warn;

use crate::runtime::JellyfinRuntime;
use crate::transport::jellyseerr::model::{
    Keyword,
    MediaType,
    ProfileTarget,
    TvDetails,
};

const ANIME_KEYWORD_ID: i32 = 210_024;

const ANIME_PROFILE: &str = "Anime 1080p";
const STANDARD_PROFILE: &str = "1080p Compact";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seasons {
    All,
    First,
    Latest,
}

impl Seasons {
    #[must_use]
    pub fn parse(raw: Option<&str>) -> Self {
        match raw {
            Some("first") => Self::First,
            Some("latest") => Self::Latest,
            Some(_) | None => Self::All,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestPlan {
    pub profile: Option<ProfileTarget>,
    pub seasons: Option<Value>,
}

pub async fn plan(
    runtime: &Arc<JellyfinRuntime>,
    kind: MediaType,
    tmdb_id: i32,
    choice: Seasons,
) -> RequestPlan {
    let (anime, available) = match kind {
        MediaType::Movie => (movie_is_anime(runtime, tmdb_id).await, Vec::new()),
        MediaType::Tv => {
            let details = tv_details(runtime, tmdb_id).await;

            (
                details.as_ref().is_some_and(|d| is_anime(&d.keywords)),
                details.as_ref().map(all_seasons).unwrap_or_default(),
            )
        },
    };

    let mut profile = resolve_profile(runtime, kind, profile_name(anime)).await;
    if profile.is_none() && anime {
        profile = resolve_profile(runtime, kind, STANDARD_PROFILE).await;
    }

    RequestPlan { profile, seasons: season_payload(kind, choice, &available) }
}

#[must_use]
pub fn is_anime(keywords: &[Keyword]) -> bool {
    keywords.iter().any(|k| k.id == ANIME_KEYWORD_ID)
}

#[must_use]
pub const fn profile_name(anime: bool) -> &'static str {
    if anime { ANIME_PROFILE } else { STANDARD_PROFILE }
}

#[must_use]
pub fn all_seasons(details: &TvDetails) -> Vec<i32> {
    details
        .seasons
        .iter()
        .map(|s| s.season_number)
        .filter(|number| *number > 0)
        .collect()
}

#[must_use]
pub fn season_payload(
    kind: MediaType,
    choice: Seasons,
    available: &[i32],
) -> Option<Value> {
    if kind == MediaType::Movie {
        return None;
    }

    if available.is_empty() {
        return Some(json!("all"));
    }

    let picked = match choice {
        Seasons::All => available.to_vec(),
        Seasons::First => available.iter().copied().min().into_iter().collect(),
        Seasons::Latest => available.iter().copied().max().into_iter().collect(),
    };

    Some(json!(picked))
}

async fn resolve_profile(
    runtime: &Arc<JellyfinRuntime>,
    kind: MediaType,
    name: &'static str,
) -> Option<ProfileTarget> {
    let key = (kind.service(), name);

    if let Some(hit) = runtime.caches.quality_profiles.get(&key).await {
        return hit;
    }

    let found = match runtime.seer.quality_profile(kind, name).await {
        Ok(found) => found,
        Err(e) => {
            warn!(
                "Could not read {} quality profiles from Jellyseerr, so the \
                 request falls back to its defaults: {e}",
                kind.service()
            );
            return None;
        },
    };

    if found.is_none() {
        warn!(
            "The {} behind Jellyseerr has no quality profile named \"{name}\", \
             so the request falls back to its defaults.",
            kind.service()
        );
    }

    runtime.caches.quality_profiles.insert(key, found).await;

    found
}

async fn tv_details(
    runtime: &Arc<JellyfinRuntime>,
    tmdb_id: i32,
) -> Option<TvDetails> {
    if let Some(hit) = runtime.caches.tv_details.get(&tmdb_id).await {
        return Some(hit);
    }

    match runtime.seer.tv(tmdb_id).await {
        Ok(details) => {
            runtime.caches.tv_details.insert(tmdb_id, details.clone()).await;
            Some(details)
        },
        Err(e) => {
            warn!("Could not read Jellyseerr tv details for {tmdb_id}: {e}");
            None
        },
    }
}

async fn movie_is_anime(runtime: &Arc<JellyfinRuntime>, tmdb_id: i32) -> bool {
    if let Some(hit) = runtime.caches.movie_details.get(&tmdb_id).await {
        return is_anime(&hit.keywords);
    }

    match runtime.seer.movie(tmdb_id).await {
        Ok(details) => {
            let anime = is_anime(&details.keywords);
            runtime.caches.movie_details.insert(tmdb_id, details).await;
            anime
        },
        Err(e) => {
            warn!("Could not read Jellyseerr movie details for {tmdb_id}: {e}");
            false
        },
    }
}
