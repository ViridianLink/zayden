use std::collections::HashMap;
use std::sync::Arc;

use gambling::{GamblingData, GameCache, HigherLower, Lotto, StaminaCron};
use llamad2::GoodMorningCache;
use music::{GuildMusic, MusicData};
use reqwest::Client as HttpClient;
use serenity::all::{Context, GenericChannelId, Guild, GuildId, Ready, UserId};
use serenity::small_fixed_array::FixedString;
use songbird::Songbird;
use sqlx::{PgPool, Postgres};
use temp_voice::{CachedState, VoiceStateCache};
use tokio::sync::RwLock;
use zayden_core::cache::GuildMembersCache;
use zayden_core::{CronJob, CronJobData, EmojiCache, EmojiCacheData};

use crate::modules::gambling::{GamblingTable, HigherLowerTable, LottoTable, StaminaTable};
use crate::sqlx_lib::PostgresPool;
use crate::{ZAYDEN_ID, ZAYDEN_TOKEN, zayden_token};

pub struct CtxData {
    pool: PgPool,
    http_client: HttpClient,
    songbird: Arc<Songbird>,
    emoji_cache: Arc<EmojiCache>,
    cron_jobs: Vec<CronJob<Postgres>>,
    voice_stats: HashMap<UserId, CachedState>,
    guild_members: HashMap<GuildId, Vec<UserId>>,
    gambling_cache: GameCache,
    last_messages: HashMap<GenericChannelId, (UserId, FixedString<u16>)>,
    music: HashMap<GuildId, GuildMusic>,
}

impl CtxData {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            http_client: HttpClient::new(),
            songbird: Songbird::serenity(),
            emoji_cache: Arc::new(EmojiCache::default()),
            cron_jobs: Vec::new(),
            voice_stats: HashMap::new(),
            guild_members: HashMap::new(),
            gambling_cache: GameCache::default(),
            last_messages: HashMap::new(),
            music: HashMap::new(),
        }
    }

    pub fn setup_static_cron(&mut self) {
        self.cron_jobs = vec![
            StaminaCron::cron_job::<Postgres, StaminaTable>(),
            Lotto::cron_job::<CtxData, Postgres, GamblingTable, LottoTable>(),
            HigherLower::cron_job::<Postgres, GamblingTable, HigherLowerTable>(),
        ];
    }

    pub async fn ready(ctx: &Context, ready: &Ready, pool: &PgPool) {
        let cache = if ready.application.id.get() == ZAYDEN_ID.get() {
            EmojiCache::new(ctx).await.unwrap()
        } else {
            let token = ZAYDEN_TOKEN.get_or_init(|| zayden_token(pool)).await;

            EmojiCache::new_from_parent(ctx, token).await.unwrap()
        };

        {
            let data = ctx.data::<RwLock<Self>>();
            let mut data = data.write().await;
            data.emoji_cache = Arc::new(cache);
        }
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

impl EmojiCacheData for CtxData {
    fn emojis(&self) -> Arc<EmojiCache> {
        Arc::clone(&self.emoji_cache)
    }

    fn emojis_mut(&mut self) -> Option<&mut EmojiCache> {
        Arc::get_mut(&mut self.emoji_cache)
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

impl GoodMorningCache for CtxData {
    fn insert(
        &mut self,
        channel_id: GenericChannelId,
        author: UserId,
        content: FixedString<u16>,
    ) -> Option<(UserId, FixedString<u16>)> {
        self.last_messages.insert(channel_id, (author, content))
    }
}

impl MusicData for CtxData {
    fn http(&self) -> HttpClient {
        self.http_client.clone()
    }

    fn songbird(&self) -> Arc<Songbird> {
        Arc::clone(&self.songbird)
    }

    fn guild_music(&self, guild: GuildId) -> Option<&GuildMusic> {
        self.music.get(&guild)
    }

    fn guild_music_mut(&mut self, guild: GuildId) -> &mut GuildMusic {
        self.music.entry(guild).or_default()
    }
}
