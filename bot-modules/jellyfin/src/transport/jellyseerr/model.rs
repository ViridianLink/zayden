use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Movie,
    Tv,
}

impl MediaType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Movie => "movie",
            Self::Tv => "tv",
        }
    }

    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "movie" => Some(Self::Movie),
            "tv" => Some(Self::Tv),
            _ => None,
        }
    }

    #[must_use]
    pub const fn service(self) -> &'static str {
        match self {
            Self::Movie => "radarr",
            Self::Tv => "sonarr",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchPage {
    #[serde(default)]
    pub total_results: i64,
    #[serde(default = "Vec::new")]
    pub results: Vec<SearchResult>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: i32,
    #[serde(default)]
    pub media_type: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub first_air_date: Option<String>,
    #[serde(default)]
    pub overview: Option<String>,
    #[serde(default)]
    pub poster_path: Option<String>,
    #[serde(default)]
    pub vote_average: Option<f64>,
    #[serde(default)]
    pub vote_count: Option<i64>,
    #[serde(default)]
    pub media_info: Option<MediaInfo>,
}

impl SearchResult {
    #[must_use]
    pub fn display_title(&self) -> &str {
        self.title.as_deref().or(self.name.as_deref()).unwrap_or("Unknown")
    }

    #[must_use]
    pub fn year(&self) -> Option<&str> {
        self.release_date
            .as_deref()
            .or(self.first_air_date.as_deref())
            .and_then(|d| d.get(..4))
            .filter(|y| !y.is_empty())
    }

    #[must_use]
    pub fn kind(&self) -> Option<MediaType> {
        self.media_type.as_deref().and_then(MediaType::parse)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    #[serde(default)]
    pub status: i32,
    #[serde(default)]
    pub tmdb_id: Option<i32>,
    #[serde(default)]
    pub imdb_id: Option<String>,
}

impl MediaInfo {
    #[must_use]
    pub const fn is_available(&self) -> bool {
        matches!(self.status, 4 | 5)
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovieDetails {
    pub id: i32,
    pub title: String,
    #[serde(default)]
    pub overview: Option<String>,
    #[serde(default)]
    pub release_date: Option<String>,
    #[serde(default)]
    pub runtime: Option<i32>,
    #[serde(default = "Vec::new")]
    pub genres: Vec<Genre>,
    #[serde(default = "Vec::new")]
    pub keywords: Vec<Keyword>,
    #[serde(default)]
    pub vote_average: Option<f64>,
    #[serde(default)]
    pub poster_path: Option<String>,
    #[serde(default)]
    pub media_info: Option<MediaInfo>,
    #[serde(default = "Vec::new")]
    pub watch_providers: Vec<RegionProviders>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TvDetails {
    pub id: i32,
    pub name: String,
    #[serde(default)]
    pub overview: Option<String>,
    #[serde(default)]
    pub first_air_date: Option<String>,
    #[serde(default)]
    pub number_of_episodes: Option<i32>,
    #[serde(default)]
    pub number_of_seasons: Option<i32>,
    #[serde(default = "Vec::new")]
    pub episode_run_time: Vec<i32>,
    #[serde(default = "Vec::new")]
    pub genres: Vec<Genre>,
    #[serde(default = "Vec::new")]
    pub keywords: Vec<Keyword>,
    #[serde(default = "Vec::new")]
    pub seasons: Vec<Season>,
    #[serde(default)]
    pub media_info: Option<MediaInfo>,
    #[serde(default = "Vec::new")]
    pub watch_providers: Vec<RegionProviders>,
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Season {
    #[serde(default)]
    pub season_number: i32,
    #[serde(default)]
    pub episode_count: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Genre {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Keyword {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeywordPage {
    #[serde(default = "Vec::new")]
    pub results: Vec<Keyword>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegionProviders {
    #[serde(rename = "iso_3166_1")]
    pub region: String,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default = "Vec::new")]
    pub flatrate: Vec<Provider>,
    #[serde(default = "Vec::new")]
    pub buy: Vec<Provider>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeerUserPage {
    #[serde(default = "Vec::new")]
    pub results: Vec<SeerUser>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeerUser {
    pub id: i32,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub jellyfin_user_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewRequest {
    pub media_type: String,
    pub media_id: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seasons: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProfileTarget {
    pub server_id: i32,
    pub profile_id: i32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceServer {
    pub id: i32,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default, rename = "is4k")]
    pub is_4k: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceProfile {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceServerDetails {
    #[serde(default = "Vec::new")]
    pub profiles: Vec<ServiceProfile>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestResult {
    pub id: i32,
    #[serde(default)]
    pub status: i32,
}

impl RequestResult {
    #[must_use]
    pub const fn is_approved(&self) -> bool {
        self.status == 2
    }

    #[must_use]
    pub const fn is_pending(&self) -> bool {
        self.status == 1
    }
}
