use jiff::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatreonPost {
    pub id: String,
    pub campaign_id: String,
    pub title: Option<String>,
    pub url: String,
    pub content_html: Option<String>,
    pub is_public: bool,
    pub published_at: Timestamp,
}
