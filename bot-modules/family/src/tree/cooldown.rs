use std::sync::LazyLock;
use std::time::{Duration, Instant};

use moka::future::Cache;
use serenity::all::{GuildId, UserId};

const RETENTION: Duration = Duration::from_secs(300);
const CAPACITY: u64 = 16_384;

static LAST_RENDER: LazyLock<Cache<(GuildId, UserId), Instant>> =
    LazyLock::new(|| {
        Cache::builder().time_to_live(RETENTION).max_capacity(CAPACITY).build()
    });

pub async fn remaining(
    guild_id: GuildId,
    user_id: UserId,
    cooldown: Option<Duration>,
) -> Option<Duration> {
    let cooldown = cooldown?;
    let last = LAST_RENDER.get(&(guild_id, user_id)).await?;

    cooldown.checked_sub(last.elapsed()).filter(|left| !left.is_zero())
}

pub async fn record(guild_id: GuildId, user_id: UserId) {
    LAST_RENDER.insert((guild_id, user_id), Instant::now()).await;
}
