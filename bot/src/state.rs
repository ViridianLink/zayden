use std::collections::HashMap;
use std::sync::Arc;

use bungie_api::{BungieClient, BungieClientBuilder};
use destiny2::endgame_analysis::EndgameAnalysisSheetCron;
use gambling::{GamblingData, GameCache, HigherLower, Lotto, StaminaCron};
use llamad2::GoodMorningCache;
use marathon::client::MarathonClient;
use marathon::cron::{MarathonAnnounceCron, MarathonNewsCron};
use music::{MusicManager, TrackResolver};
use palworld::client::PalworldClient;
use palworld::cron::PalworldUploadSweepCron;
use palworld::transport::Pelican;
use serenity::all::{Context, GenericChannelId, Guild, GuildId, Ready, UserId};
use songbird::Songbird;
use sqlx::{PgPool, Postgres};
use temp_voice::VoiceStateCache;
use tokio::sync::RwLock;
use zayden_app::config::BotConfig;
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

pub struct BotState {
    pub app: Arc<AppState>,
    pub songbird: Arc<Songbird>,
    pub music: Arc<MusicManager>,
    pub music_resolver: Arc<dyn TrackResolver>,
    pub voice_states: Arc<VoiceStateCache>,
    pub marathon: Arc<MarathonClient>,
    pub palworld: Arc<PalworldClient>,
    pub bungie_client: Arc<BungieClient>,
    marathon_bungie_api_key: String,
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

        let marathon = Arc::new(MarathonClient::new(
            app.http.clone(),
            config.flaresolverr_url.clone(),
        ));

        let pelican = config.pelican.clone().map(|p| {
            Pelican::new(
                app.http.clone(),
                p.base_url,
                p.api_key,
                p.server_id,
                p.save_path,
            )
        });

        let palworld = Arc::new(PalworldClient::new(
            app.http.clone(),
            config.flaresolverr_url.clone(),
            config.palworld_paldex_url.clone(),
            config.palworld_palcalc_url.clone(),
            config.palworld_save_dir.clone(),
            config.palworld_uploads_dir.clone(),
            pelican,
        ));

        Ok(Self {
            app,
            songbird: Songbird::serenity(),
            music: Arc::new(MusicManager::new()),
            music_resolver,
            voice_states: Arc::new(VoiceStateCache::new()),
            marathon,
            palworld,
            bungie_client: Arc::new(bungie_client),
            marathon_bungie_api_key: config.bungie_api_key.clone(),
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
            EndgameAnalysisSheetCron::cron_job(
                Arc::clone(&self.bungie_client),
                self.app.google_api_key.clone(),
            ),
            MarathonAnnounceCron::cron_job(Arc::clone(&self.marathon)),
            MarathonNewsCron::cron_job(
                self.app.http.clone(),
                Some(self.marathon_bungie_api_key.clone()),
            ),
            PalworldUploadSweepCron::cron_job(),
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
