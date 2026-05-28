use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use bungie_api::{BungieClient, BungieClientBuilder};
use destiny2_core::BungieClientData;
use gambling::{GamblingData, GameCache, HigherLower, Lotto, StaminaCron};
use llamad2::GoodMorningCache;
use serenity::all::{Context, GenericChannelId, Guild, GuildId, Ready, UserId};
use sqlx::{PgPool, Postgres};
use temp_voice::{CachedState, VoiceStateCache};
use tokio::sync::RwLock;
use zayden_app::config::BotConfig;
use zayden_app::state::AppState;
use zayden_core::cache::GuildMembersCache;
use zayden_core::{CronJob, CronJobData, EmojiCache, EmojiCacheData};

use crate::bindings::gambling::{GamblingTable, HigherLowerTable, LottoTable, StaminaTable};
use crate::{ZAYDEN_ID, ZAYDEN_TOKEN, zayden_token};

/// Bot-specific application state stored in Serenity's context data.
///
/// Wraps the shared [`AppState`] and adds Discord-gateway-only caches.
/// Stored as `Arc<RwLock<BotState>>` in `ctx.data`.
pub struct BotState {
    pub app: Arc<AppState>,
    pub started_cron: AtomicBool,
    bungie_client: BungieClient,
    emoji_cache: Arc<EmojiCache>,
    cron_jobs: Vec<CronJob<Postgres>>,
    voice_stats: HashMap<UserId, CachedState>,
    guild_members: HashMap<GuildId, Vec<UserId>>,
    gambling_cache: GameCache,
    good_morning_cache: HashMap<GenericChannelId, (UserId, bool)>,
}

impl BotState {
    pub fn new(app: Arc<AppState>, config: &BotConfig) -> Self {
        let bungie_client = BungieClientBuilder::new(config.bungie_api_key.clone())
            .build()
            .expect("BungieClient construction failed — check BUNGIE_API_KEY");

        Self {
            app,
            started_cron: AtomicBool::new(false),
            bungie_client,
            emoji_cache: Arc::default(),
            cron_jobs: Vec::new(),
            voice_stats: HashMap::new(),
            guild_members: HashMap::new(),
            gambling_cache: GameCache::default(),
            good_morning_cache: HashMap::new(),
        }
    }

    pub fn setup_static_cron(&mut self) {
        self.cron_jobs = vec![
            StaminaCron::cron_job::<Postgres, StaminaTable>(),
            Lotto::cron_job::<BotState, Postgres, GamblingTable, LottoTable>(),
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

        let data = ctx.data::<RwLock<Self>>();
        let mut data = data.write().await;
        data.emoji_cache = Arc::new(cache);
    }

    pub async fn guild_create(data: Arc<RwLock<Self>>, guild: &Guild) {
        let mut data = data.write().await;
        VoiceStateCache::guild_create(&mut *data, guild);
        GuildMembersCache::guild_create(&mut *data, guild);
    }
}

// --- Trait implementations (trivial field accessors) ---

impl BungieClientData for BotState {
    fn bungie_client(&self) -> &BungieClient {
        &self.bungie_client
    }
}

impl EmojiCacheData for BotState {
    fn emojis(&self) -> Arc<EmojiCache> {
        Arc::clone(&self.emoji_cache)
    }

    fn emojis_mut(&mut self) -> Option<&mut EmojiCache> {
        Arc::get_mut(&mut self.emoji_cache)
    }
}

impl CronJobData<Postgres> for BotState {
    fn jobs(&self) -> &[CronJob<Postgres>] {
        &self.cron_jobs
    }

    fn jobs_mut(&mut self) -> &mut Vec<CronJob<Postgres>> {
        &mut self.cron_jobs
    }
}

impl VoiceStateCache for BotState {
    fn get(&self) -> &HashMap<UserId, CachedState> {
        &self.voice_stats
    }

    fn get_mut(&mut self) -> &mut HashMap<UserId, CachedState> {
        &mut self.voice_stats
    }
}

impl GuildMembersCache for BotState {
    fn get(&self) -> &HashMap<GuildId, Vec<UserId>> {
        &self.guild_members
    }

    fn get_mut(&mut self) -> &mut HashMap<GuildId, Vec<UserId>> {
        &mut self.guild_members
    }
}

impl GamblingData for BotState {
    fn game_cache(&self) -> &GameCache {
        &self.gambling_cache
    }

    fn game_cache_mut(&mut self) -> &mut GameCache {
        &mut self.gambling_cache
    }
}

impl GoodMorningCache for BotState {
    fn insert(
        &mut self,
        channel_id: GenericChannelId,
        author: UserId,
        is_good_morning: bool,
    ) -> Option<(UserId, bool)> {
        self.good_morning_cache
            .insert(channel_id, (author, is_good_morning))
    }
}
