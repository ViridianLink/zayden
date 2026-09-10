use serde::{Deserialize, Serialize};

pub const TICKS_PER_SECOND: i64 = 10_000_000;

#[must_use]
pub const fn ticks_to_seconds(ticks: i64) -> i64 {
    ticks / TICKS_PER_SECOND
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ItemsPage<T> {
    #[serde(default = "Vec::new")]
    pub items: Vec<T>,
    #[serde(default)]
    pub total_record_count: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ProviderIds {
    pub tmdb: Option<String>,
    pub imdb: Option<String>,
    pub tvdb: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Item {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub sort_name: Option<String>,
    #[serde(rename = "Type", default)]
    pub item_type: Option<String>,
    #[serde(default)]
    pub series_name: Option<String>,
    #[serde(default)]
    pub series_id: Option<String>,
    #[serde(default)]
    pub parent_index_number: Option<i32>,
    #[serde(default)]
    pub index_number: Option<i32>,
    #[serde(default)]
    pub production_year: Option<i32>,
    #[serde(default)]
    pub run_time_ticks: Option<i64>,
    #[serde(default)]
    pub community_rating: Option<f32>,
    #[serde(default)]
    pub genres: Option<Vec<String>>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub date_created: Option<String>,
    #[serde(default)]
    pub provider_ids: Option<ProviderIds>,
}

#[derive(Debug, Clone, Copy)]
pub struct LibraryCounts {
    pub movies: i64,
    pub series: i64,
    pub episodes: i64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct QuickConnectState {
    pub secret: String,
    pub code: String,
    #[serde(default)]
    pub authenticated: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthenticationResult {
    pub access_token: String,
    pub user: AuthenticatedUser,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct AuthenticatedUser {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct User {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct VirtualFolder {
    pub name: String,
    #[serde(default)]
    pub item_id: Option<String>,
    #[serde(default)]
    pub collection_type: Option<String>,
    #[serde(default = "Vec::new")]
    pub locations: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserPolicy {
    pub is_administrator: bool,
    pub is_hidden: bool,
    pub is_disabled: bool,
    pub enable_all_folders: bool,
    pub enabled_folders: Vec<String>,
    pub enable_content_downloading: bool,
    pub enable_user_preference_access: bool,
    pub enable_remote_control_of_other_users: bool,
    pub enable_shared_device_control: bool,
    pub enable_media_playback: bool,
    pub enable_playback_remuxing: bool,
    pub enable_all_devices: bool,
    pub enable_public_sharing: bool,
    pub enable_collection_management: bool,
    pub enable_subtitle_management: bool,
    pub enable_live_tv_access: bool,
    pub enable_live_tv_management: bool,
}

impl UserPolicy {
    #[must_use]
    pub fn guest(library_item_id: String) -> Self {
        Self {
            is_administrator: false,
            is_hidden: true,
            is_disabled: false,
            enable_all_folders: false,
            enabled_folders: vec![library_item_id],
            enable_content_downloading: false,
            enable_user_preference_access: false,
            enable_remote_control_of_other_users: false,
            enable_shared_device_control: false,
            enable_media_playback: true,
            enable_playback_remuxing: true,
            enable_all_devices: true,
            enable_public_sharing: false,
            enable_collection_management: false,
            enable_subtitle_management: false,
            enable_live_tv_access: false,
            enable_live_tv_management: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct NewUser {
    pub name: String,
    pub password: String,
}
