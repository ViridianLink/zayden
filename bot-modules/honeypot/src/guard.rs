use std::sync::{Arc, LazyLock};
use std::time::Duration;

use moka::future::Cache;
use serenity::all::{Context, GuildId, UserId};

use crate::error::Result;
use crate::policy::GuildFacts;

const FACTS_TTL: Duration = Duration::from_mins(5);
const ACTION_TTL: Duration = Duration::from_mins(1);

pub struct HoneypotGuard {
    facts: Cache<GuildId, Arc<GuildFacts>>,
    recent: Cache<(GuildId, UserId), ()>,
}

impl HoneypotGuard {
    fn new() -> Self {
        Self {
            facts: Cache::builder()
                .time_to_live(FACTS_TTL)
                .max_capacity(1_024)
                .build(),
            recent: Cache::builder()
                .time_to_live(ACTION_TTL)
                .max_capacity(4_096)
                .build(),
        }
    }

    pub async fn facts(
        &self,
        ctx: &Context,
        guild_id: GuildId,
    ) -> Result<Arc<GuildFacts>> {
        if let Some(cached) = self.facts.get(&guild_id).await {
            return Ok(cached);
        }

        let guild = guild_id.to_partial_guild(&ctx.http).await?;

        let facts = Arc::new(GuildFacts {
            owner_id: guild.owner_id,
            role_perms: guild
                .roles
                .iter()
                .map(|role| (role.id, role.permissions))
                .collect(),
            everyone_role: guild_id.everyone_role(),
        });

        self.facts.insert(guild_id, Arc::clone(&facts)).await;

        Ok(facts)
    }

    pub async fn claim(&self, guild_id: GuildId, user_id: UserId) -> bool {
        let key = (guild_id, user_id);

        if self.recent.get(&key).await.is_some() {
            return false;
        }

        self.recent.insert(key, ()).await;

        true
    }

    pub async fn release(&self, guild_id: GuildId, user_id: UserId) {
        self.recent.invalidate(&(guild_id, user_id)).await;
    }

    pub async fn forget(&self, guild_id: GuildId) {
        self.facts.invalidate(&guild_id).await;
    }
}

pub static GUARD: LazyLock<HoneypotGuard> = LazyLock::new(HoneypotGuard::new);
