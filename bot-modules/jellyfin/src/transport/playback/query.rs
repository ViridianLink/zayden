pub const MIN_PLAY_SECONDS: i64 = 60;

#[must_use]
pub fn daily_rollup(since_days: i64) -> String {
    format!(
        "SELECT UserId, date(DateCreated) AS day, \
         SUM(PlayDuration) AS seconds, COUNT(*) AS items \
         FROM PlaybackActivity \
         WHERE PlayDuration >= {MIN_PLAY_SECONDS} \
         AND DateCreated >= date('now', '-{since_days} days') \
         GROUP BY UserId, day"
    )
}

#[must_use]
pub fn recent_plays(jellyfin_user_id: &str, limit: i64) -> String {
    format!(
        "SELECT ItemId, ItemName, ItemType, DateCreated \
         FROM PlaybackActivity \
         WHERE UserId = '{}' AND PlayDuration >= {MIN_PLAY_SECONDS} \
         ORDER BY DateCreated DESC LIMIT {limit}",
        escape(jellyfin_user_id)
    )
}

#[must_use]
pub fn server_recent_plays(limit: i64) -> String {
    format!(
        "SELECT ItemId, ItemName, ItemType, DateCreated \
         FROM PlaybackActivity \
         WHERE PlayDuration >= {MIN_PLAY_SECONDS} \
         ORDER BY DateCreated DESC LIMIT {limit}"
    )
}

#[must_use]
pub fn item_play_counts() -> String {
    format!(
        "SELECT ItemId, COUNT(*) AS plays, COUNT(DISTINCT UserId) AS users \
         FROM PlaybackActivity \
         WHERE PlayDuration >= {MIN_PLAY_SECONDS} \
         GROUP BY ItemId"
    )
}

#[must_use]
pub fn escape(value: &str) -> String {
    value.replace('\'', "''")
}
