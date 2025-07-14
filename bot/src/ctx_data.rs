use std::collections::HashMap;
use std::sync::Arc;

use gambling::{GamblingData, GameCache, HigherLower, Lotto, StaminaCron};
use serenity::all::{Guild, GuildId, UserId};
use sqlx::{PgPool, Postgres};
use temp_voice::{CachedState, VoiceStateCache};
use tokio::sync::RwLock;
use zayden_core::cache::GuildMembersCache;
use zayden_core::{CronJob, CronJobData};

use crate::modules::gambling::{GamblingTable, HigherLowerTable, LottoTable, StaminaTable};
use crate::sqlx_lib::PostgresPool;

pub struct CtxData {
    pool: PgPool,
    cron_jobs: Vec<CronJob<Postgres>>,
    voice_stats: HashMap<UserId, CachedState>,
    guild_members: HashMap<GuildId, Vec<UserId>>,
    gambling_cache: GameCache,
}

impl CtxData {
    pub async fn new() -> sqlx::Result<Self> {
        let pool = Self::new_pool().await?;

        Ok(Self {
            pool,
            cron_jobs: vec![
                Lotto::cron_job::<Postgres, GamblingTable, LottoTable>(),
                HigherLower::cron_job::<Postgres, GamblingTable, HigherLowerTable>(),
                StaminaCron::cron_job::<Postgres, StaminaTable>(),
            ],
            voice_stats: HashMap::new(),
            guild_members: HashMap::new(),
            gambling_cache: GameCache::default(),
        })
    }

    pub async fn guild_create(data: Arc<RwLock<Self>>, guild: &Guild) {
        let mut data = data.write().await;
        VoiceStateCache::guild_create(&mut *data, guild);
        GuildMembersCache::guild_create(&mut *data, guild);
    }
}

impl PostgresPool for CtxData {
    fn pool(&self) -> &PgPool {
        &self.pool
    }
}

impl CronJobData<Postgres> for CtxData {
    fn jobs(&self) -> &[CronJob<Postgres>] {
        &self.cron_jobs
    }

    fn jobs_mut(&mut self) -> &mut Vec<CronJob<Postgres>> {
        &mut self.cron_jobs
    }
}

impl VoiceStateCache for CtxData {
    fn get(&self) -> &HashMap<UserId, CachedState> {
        &self.voice_stats
    }

    fn get_mut(&mut self) -> &mut HashMap<UserId, CachedState> {
        &mut self.voice_stats
    }
}

impl GuildMembersCache for CtxData {
    fn get(&self) -> &HashMap<GuildId, Vec<UserId>> {
        &self.guild_members
    }

    fn get_mut(&mut self) -> &mut HashMap<GuildId, Vec<UserId>> {
        &mut self.guild_members
    }
}

impl GamblingData for CtxData {
    fn game_cache(&self) -> &GameCache {
        &self.gambling_cache
    }

    fn game_cache_mut(&mut self) -> &mut GameCache {
        &mut self.gambling_cache
    }
}
