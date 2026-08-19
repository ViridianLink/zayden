use std::sync::LazyLock;
use std::time::Duration;

use jiff::{SignedDuration, Timestamp};
use moka::future::Cache;
use serenity::all::{GuildId, UserId};

const RETENTION: Duration = Duration::from_secs(300);
const CAPACITY: u64 = 16_384;

static LAST_RENDER: LazyLock<Cache<(GuildId, UserId), Timestamp>> =
    LazyLock::new(|| {
        Cache::builder().time_to_live(RETENTION).max_capacity(CAPACITY).build()
    });

pub async fn remaining(
    guild_id: GuildId,
    user_id: UserId,
    cooldown: Option<SignedDuration>,
) -> Option<SignedDuration> {
    let cooldown = cooldown?;
    let last = LAST_RENDER.get(&(guild_id, user_id)).await?;

    cooldown
        .checked_sub(Timestamp::now().duration_since(last))
        .filter(SignedDuration::is_positive)
}

pub async fn record(guild_id: GuildId, user_id: UserId) {
    LAST_RENDER.insert((guild_id, user_id), Timestamp::now()).await;
}
