use jiff::Timestamp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YoutubeVideo {
    pub id: String,
    pub channel_id: String,
    pub title: String,
    pub published_at: Timestamp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnChannel {
    pub id: String,
    pub title: String,
    pub uploads_playlist_id: String,
}

#[must_use]
pub fn video_url(video_id: &str) -> String {
    format!("https://www.youtube.com/watch?v={video_id}")
}
