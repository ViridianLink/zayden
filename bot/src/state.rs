use std::collections::HashMap;
use std::sync::Arc;

use bungie_api::{BungieClient, BungieClientBuilder};
use destiny2_core::BungieClientData;
use gambling::{GamblingData, GameCache, HigherLower, Lotto, StaminaCron};
use llamad2::GoodMorningCache;
use music::{MusicManager, MusicSettingsRow, TrackResolver};
use serenity::all::{Context, GenericChannelId, Guild, GuildId, Ready, UserId};
use songbird::Songbird;
use sqlx::{PgPool, Postgres};
use temp_voice::VoiceStateCache;
use tokio::sync::RwLock;
use zayden_app::config::{BotConfig, SettingsStore};
use zayden_app::state::AppState;
use zayden_core::cache::GuildMembersCache;
use zayden_core::{CronJob, CronJobData, EmojiCache, EmojiCacheData};

use crate::bindings::gambling::{
    GamblingTable,
    HigherLowerTable,
    LottoTable,
    StaminaTable,
};
use crate::{Result, ZAYDEN_TOKEN, zayden_token};

/// Bot-specific application state stored in Serenity's context data.
pub struct BotState {
    pub app: Arc<AppState>,
    pub songbird: Arc<Songbird>,
    pub music: Arc<MusicManager>,
    pub music_settings: Arc<SettingsStore<MusicSettingsRow>>,
    pub music_resolver: Arc<dyn TrackResolver>,
    pub voice_states: Arc<VoiceStateCache>,
    bungie_client: Arc<BungieClient>,
    emoji_cache: Arc<EmojiCache>,
    cron_jobs: Vec<CronJob<Postgres>>,
    guild_members: HashMap<GuildId, Vec<UserId>>,
    gambling_cache: GameCache,
    good_morning_cache: HashMap<GenericChannelId, (UserId, bool)>,
}

impl BotState {
    pub fn new(
        app: Arc<AppState>,
        config: &BotConfig,
        music_resolver: Arc<dyn TrackResolver>,
    ) -> std::result::Result<Self, bungie_api::BungieApiError> {
        let bungie_client =
            BungieClientBuilder::new(config.bungie_api_key.clone()).build()?;

        let music_settings =
            Arc::new(SettingsStore::new(app.db.clone(), app.events.clone()));
        SettingsStore::spawn_invalidator(
            Arc::clone(&music_settings),
            app.subscribe(),
        );

        Ok(Self {
            app,
            songbird: Songbird::serenity(),
            music: Arc::new(MusicManager::new()),
            music_settings,
            music_resolver,
            voice_states: Arc::new(VoiceStateCache::new()),
            bungie_client: Arc::new(bungie_client),
            emoji_cache: Arc::default(),
            cron_jobs: Vec::new(),
            guild_members: HashMap::new(),
            gambling_cache: GameCache::default(),
            good_morning_cache: HashMap::new(),
        })
    }

    pub fn setup_static_cron(&mut self) {
        let jobs = [
            StaminaCron::cron_job::<Postgres, StaminaTable>(),
            Lotto::cron_job::<Self, Postgres, GamblingTable, LottoTable>(),
            HigherLower::cron_job::<Postgres, GamblingTable, HigherLowerTable>(),
            endgame_analysis::EndgameAnalysisSheetCron::cron_job::<Postgres>(
                Arc::clone(&self.bungie_client),
                self.app.google_api_key.clone(),
            ),
        ];
        for job in jobs {
            match job {
                Ok(j) => self.cron_jobs.push(j),
                Err(e) => {
                    tracing::error!(error = ?e, "failed to create cron job");
                },
            }
        }
    }

    pub async fn ready(
        ctx: &Context,
        ready: &Ready,
        pool: &PgPool,
        zayden_id: u64,
    ) -> Result<()> {
        let cache = if ready.application.id.get() == zayden_id {
            EmojiCache::new(ctx).await?
        } else {
            let token = ZAYDEN_TOKEN.get_or_try_init(|| zayden_token(pool)).await?;
            EmojiCache::new_from_parent(ctx, token).await?
        };

        let data = ctx.data::<RwLock<Self>>();
        let mut data = data.write().await;
        data.emoji_cache = Arc::new(cache);
        drop(data);
        Ok(())
    }

    pub async fn guild_create(data: Arc<RwLock<Self>>, guild: &Guild) {
        let mut data = data.write().await;
        data.voice_states.guild_create(guild);
        GuildMembersCache::guild_create(&mut *data, guild);
        data.music.occupancy().guild_create(guild);
    }
}

// --- Trait implementations (trivial field accessors) ---

impl BungieClientData for BotState {
    fn bungie_client(&self) -> Arc<BungieClient> {
        Arc::clone(&self.bungie_client)
    }
}

impl EmojiCacheData for BotState {
    fn emojis(&self) -> Arc<EmojiCache> {
        Arc::clone(&self.emoji_cache)
    }

    fn emojis_mut(&mut self) -> &mut EmojiCache {
        Arc::make_mut(&mut self.emoji_cache)
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
}

impl GoodMorningCache for BotState {
    fn insert(
        &mut self,
        channel_id: GenericChannelId,
        author: UserId,
        is_good_morning: bool,
    ) -> Option<(UserId, bool)> {
        self.good_morning_cache.insert(channel_id, (author, is_good_morning))
    }
}
