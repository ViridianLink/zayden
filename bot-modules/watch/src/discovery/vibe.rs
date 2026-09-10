use std::sync::Arc;

use ai::chat::{Message as ChatMessage, Role};
use ai::openai::AiClient;
use futures::future;
use jellyfin::JellyfinError;
use jellyfin::runtime::JellyfinRuntime;
use jellyfin::transport::jellyseerr::discover::DiscoverFilters;
use jellyfin::transport::jellyseerr::model::{MediaType, SearchResult};
use serde::Deserialize;
use zayden_app::state::AppState;

use crate::error::{Result, WatchError};

const SYSTEM_PROMPT: &str = "You translate a description of a film mood into \
TMDB search facets for a media library bot.

Rules:
- `genres` must be official TMDB genre names (for example: Action, Adventure, \
Animation, Comedy, Crime, Documentary, Drama, Family, Fantasy, History, Horror, \
Music, Mystery, Romance, Science Fiction, TV Movie, Thriller, War, Western).
- `keywords` must be short, real TMDB keyword phrases such as \"twist ending\", \
\"cyberpunk\", \"heist\", \"time loop\". Give at most four, most important first.
- Never repeat a keyword, and never pad the list.
- `year_from` and `year_to` are 0 when the user gave no period.
- Answer with the JSON object only.";

const SCHEMA_NAME: &str = "vibe_facets";
const MAX_TOKENS: u32 = 400;
const TEMPERATURE: f32 = 0.3;
const MAX_KEYWORDS: usize = 4;
const MIN_RESULTS: usize = 3;

#[derive(Debug, Clone, Deserialize)]
pub struct VibeFacets {
    #[serde(default)]
    pub genres: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub year_from: i32,
    #[serde(default)]
    pub year_to: i32,
}

fn schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "genres": { "type": "array", "items": { "type": "string" } },
            "keywords": { "type": "array", "items": { "type": "string" } },
            "year_from": { "type": "integer" },
            "year_to": { "type": "integer" }
        },
        "required": ["genres", "keywords", "year_from", "year_to"],
        "additionalProperties": false
    })
}

#[must_use]
pub fn genre_id(name: &str) -> Option<i32> {
    Some(match name.trim().to_lowercase().as_str() {
        "action" => 28,
        "adventure" => 12,
        "animation" => 16,
        "comedy" => 35,
        "crime" => 80,
        "documentary" => 99,
        "drama" => 18,
        "family" => 10_751,
        "fantasy" => 14,
        "history" => 36,
        "horror" => 27,
        "music" => 10_402,
        "mystery" => 9_648,
        "romance" => 10_749,
        "science fiction" | "sci-fi" | "scifi" => 878,
        "tv movie" => 10_770,
        "thriller" => 53,
        "war" => 10_752,
        "western" => 37,
        _ => return None,
    })
}

pub async fn facets(app: &AppState, description: &str) -> Result<VibeFacets> {
    if app.ai_provider_key.is_empty() {
        return Err(JellyfinError::AiUnavailable.into());
    }

    let client = AiClient::new(
        &app.ai_provider_key,
        &app.ai_api_endpoint,
        &app.ai_model_structured,
    )
    .map_err(|e| WatchError::Jellyfin(JellyfinError::Ai(e.to_string())))?;

    let messages = vec![
        ChatMessage::new(Role::System, SYSTEM_PROMPT),
        ChatMessage::new(Role::User, description),
    ];

    let mut facets: VibeFacets = client
        .chat_json(messages, MAX_TOKENS, Some(TEMPERATURE), SCHEMA_NAME, schema())
        .await
        .map_err(|e| WatchError::Jellyfin(JellyfinError::Ai(e.to_string())))?;

    // A model that starts repeating itself would otherwise produce a filter so
    // narrow nothing can match.
    facets.keywords = dedupe(facets.keywords, MAX_KEYWORDS);
    facets.genres = dedupe(facets.genres, MAX_KEYWORDS);

    Ok(facets)
}

fn dedupe(values: Vec<String>, limit: usize) -> Vec<String> {
    let mut seen = Vec::with_capacity(limit);

    for value in values {
        let trimmed = value.trim().to_lowercase();
        if trimmed.is_empty() || seen.contains(&trimmed) {
            continue;
        }
        seen.push(trimmed);
        if seen.len() == limit {
            break;
        }
    }

    seen
}

pub async fn to_filters(
    runtime: &Arc<JellyfinRuntime>,
    facets: &VibeFacets,
) -> Result<DiscoverFilters> {
    let lookups = facets.keywords.iter().map(|name| keyword_id(runtime, name));
    let keywords = future::join_all(lookups).await.into_iter().flatten().collect();

    Ok(DiscoverFilters {
        keywords,
        genres: facets.genres.iter().filter_map(|g| genre_id(g)).collect(),
        exclude_genres: Vec::new(),
        year_from: (facets.year_from > 1800).then_some(facets.year_from),
        year_to: (facets.year_to > 1800).then_some(facets.year_to),
        min_votes: None,
        sort_by: None,
    })
}

async fn keyword_id(runtime: &Arc<JellyfinRuntime>, name: &str) -> Option<i32> {
    if let Some(cached) = runtime.caches.keyword_ids.get(name).await {
        return cached;
    }

    let resolved = runtime.seer.keyword_id(name).await.ok().flatten();
    runtime.caches.keyword_ids.insert(name.to_owned(), resolved).await;

    resolved
}

pub async fn search(
    runtime: &Arc<JellyfinRuntime>,
    filters: DiscoverFilters,
) -> Result<(Vec<SearchResult>, bool)> {
    let mut current = filters;
    let mut relaxed = false;

    loop {
        let results = runtime
            .seer
            .discover(MediaType::Movie, &current)
            .await
            .map_err(JellyfinError::from)?;

        if results.len() >= MIN_RESULTS {
            return Ok((results, relaxed));
        }

        match current.relaxed() {
            Some(next) => {
                current = next;
                relaxed = true;
            },
            None => return Ok((results, relaxed)),
        }
    }
}
