use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct LeaderboardEntry {
    pub(crate) rank: i64,
    pub(crate) user_id: String,
    pub(crate) name: String,
    pub(crate) avatar: Option<String>,
    pub(crate) level: i32,
    pub(crate) xp: i32,
    pub(crate) message_count: i64,
}
