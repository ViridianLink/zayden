use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct QueryResponse {
    #[serde(rename = "colums", default = "Vec::new")]
    pub columns: Vec<String>,
    #[serde(default = "Vec::new")]
    pub results: Vec<Vec<String>>,
    #[serde(default)]
    pub message: String,
}

impl QueryResponse {
    #[must_use]
    pub fn column(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyPlay {
    pub jellyfin_user_id: String,
    pub day: String,
    pub seconds: i64,
    pub items: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentPlay {
    pub item_id: String,
    pub item_name: String,
    pub item_type: String,
    pub played_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemPlayCount {
    pub item_id: String,
    pub plays: i64,
    pub distinct_users: i64,
}
