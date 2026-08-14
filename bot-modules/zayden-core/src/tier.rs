use std::sync::LazyLock;
use std::time::Duration;

use moka::future::Cache;
use serenity::all::{GuildId, Http, UserId};
use tracing::warn;
use zayden_app::entitlement::{EntitlementService, Tier};

const OWNER_TTL: Duration = Duration::from_mins(30);
const OWNER_CAPACITY: u64 = 4_096;

static OWNERS: LazyLock<Cache<GuildId, UserId>> = LazyLock::new(|| {
    Cache::builder().time_to_live(OWNER_TTL).max_capacity(OWNER_CAPACITY).build()
});

pub async fn guild_owner(
    http: &Http,
    guild_id: GuildId,
) -> serenity::Result<UserId> {
    if let Some(owner) = OWNERS.get(&guild_id).await {
        return Ok(owner);
    }

    let owner = guild_id.to_partial_guild(http).await?.owner_id;
    OWNERS.insert(guild_id, owner).await;

    Ok(owner)
}

pub async fn invalidate_guild_owner(guild_id: GuildId) {
    OWNERS.invalidate(&guild_id).await;
}

pub async fn server_tier(
    http: &Http,
    entitlements: &EntitlementService,
    guild_id: GuildId,
) -> Tier {
    match guild_owner(http, guild_id).await {
        Ok(owner) => entitlements.server_tier(guild_id.get(), owner.get()).await,
        Err(e) => {
            warn!(
                error = ?e,
                %guild_id,
                "guild owner lookup failed; falling back to the guild's own tier",
            );
            entitlements.guild_tier(guild_id.get()).await
        },
    }
}
