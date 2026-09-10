use std::sync::Arc;

use bungie_api::{BungieClient, BungieClientBuilder};
use dashmap::DashMap;
use destiny2::endgame_analysis::EndgameAnalysisSheetCron;
use gambling::{GamblingData, GameCache, HigherLower, Lotto, StaminaCron};
use hosting::HostingRuntime;
use hosting::cron::{HostingDeleteCron, HostingExpireCron, HostingReminderCron};
use jellyfin::cron::{JellyfinIndexRefreshCron, JellyfinRollupCron};
use jellyfin::runtime::JellyfinRuntime;
use llamad2::GoodMorningCache;
use marathon::client::MarathonClient;
use marathon::cron::{MarathonAnnounceCron, MarathonNewsCron};
use music::{MusicManager, TrackResolver};
use palworld::client::PalworldClient;
use palworld::cron::{
    PalworldSaveRefreshCron,
    PalworldUploadSweepCron,
    PalworldWarmCron,
};
use palworld::transport::Pelican;
use patreon::PatreonPollCron;
use patreon::oauth::PatreonApp;
use serenity::all::{Context, GenericChannelId, Guild, GuildId, Ready, UserId};
use songbird::Songbird;
use sqlx::PgPool;
use temp_voice::VoiceStateCache;
use ticket::{
    SupportIdleCloseCron,
    SupportIdleCron,
    SupportIdleGcCron,
    SupportIdleStaleCron,
    WikiIndex,
};
use tokio::sync::RwLock;
use watch::JellyfinPartyReaperCron;
use zayden_app::config::BotConfig;
use zayden_app::state::AppState;
use zayden_core::cache::GuildMembersCache;
use zayden_core::{CronJob, CronJobData, EmojiCache, EmojiCacheData};

use crate::cron::EntitlementSweepCron;
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
    pub wiki_index: Arc<WikiIndex>,
    pub jellyfin: Option<Arc<JellyfinRuntime>>,
    pub hosting: Option<Arc<HostingRuntime>>,
    marathon_bungie_api_key: String,
    pub patreon: Option<Arc<PatreonApp>>,
    emoji_cache: Arc<EmojiCache>,
    cron_jobs: Vec<CronJob>,
    guild_members: DashMap<GuildId, Vec<UserId>>,
    gambling_cache: GameCache,
    good_morning_cache: DashMap<GenericChannelId, (UserId, bool)>,
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

        let pelican = config.pelican.clone().and_then(|p| {
            let save = p.save?;
            Some(Pelican::new(
                app.http.clone(),
                p.base_url,
                p.api_key,
                save.server_id,
                save.save_path,
            ))
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

        let patreon = config.patreon.as_ref().map(|p| {
            Arc::new(PatreonApp {
                client_id: p.client_id.clone(),
                client_secret: p.client_secret.clone(),
                redirect_uri: p.redirect_uri.clone(),
            })
        });

        let jellyfin = config
            .jellyfin
            .as_ref()
            .map(|c| JellyfinRuntime::new(app.http.clone(), c));

        // Hosting needs the panel's admin-scoped key, so its client is built
        // here and never handed to the dashboard.
        let hosting = config.pelican.as_ref().zip(config.hosting.as_ref()).map(
            |(pelican, hosting)| {
                let runtime = Arc::new(HostingRuntime::new(
                    app.http.clone(),
                    &pelican.base_url,
                    &pelican.api_key,
                    hosting.clone(),
                ));
                HostingRuntime::spawn_paid_listener(
                    Arc::clone(&runtime),
                    app.db.clone(),
                    app.subscribe(),
                );
                runtime
            },
        );

        let wiki_index = Arc::new(WikiIndex::new(app.http.clone()));
        WikiIndex::spawn_invalidator(Arc::clone(&wiki_index), app.subscribe());

        Ok(Self {
            app,
            songbird: Songbird::serenity(),
            music: Arc::new(MusicManager::new()),
            music_resolver,
            voice_states: Arc::new(VoiceStateCache::new()),
            marathon,
            palworld,
            bungie_client: Arc::new(bungie_client),
            wiki_index,
            jellyfin,
            hosting,
            marathon_bungie_api_key: config.bungie_api_key.clone(),
            patreon,
            emoji_cache: Arc::default(),
            cron_jobs: Vec::new(),
            guild_members: DashMap::new(),
            gambling_cache: GameCache::default(),
            good_morning_cache: DashMap::new(),
        })
    }

    pub fn setup_static_cron(&mut self) {
        // Patreon is optional: without an app registration no guild can connect,
        // so the poll would have nothing to do.
        if let Some(app) = self.patreon.as_ref() {
            match PatreonPollCron::cron_job(self.app.http.clone(), Arc::clone(app)) {
                Ok(job) => self.cron_jobs.push(job),
                Err(e) => tracing::error!(error = ?e, "failed to create cron job"),
            }
        }

        // Guest accounts are real accounts on a real media server, so the
        // reaper is registered alongside the refresh jobs rather than being
        // left to the in-memory per-party schedules.
        if let Some(runtime) = self.jellyfin.as_ref() {
            let jellyfin_jobs = [
                JellyfinIndexRefreshCron::cron_job(Arc::clone(runtime)),
                JellyfinRollupCron::cron_job(Arc::clone(runtime)),
                JellyfinPartyReaperCron::cron_job(Arc::clone(runtime)),
                JellyfinPartyReaperCron::reconcile_job(Arc::clone(runtime)),
            ];

            for job in jellyfin_jobs {
                match job {
                    Ok(j) => self.cron_jobs.push(j),
                    Err(e) => {
                        tracing::error!(error = ?e, "failed to create cron job");
                    },
                }
            }
        }

        // Hosting is optional; without a catalog and panel credentials there
        // are no servers for these sweeps to act on.
        if let Some(runtime) = self.hosting.as_ref() {
            let hosting_jobs = [
                HostingReminderCron::cron_job(Arc::clone(runtime)),
                HostingExpireCron::cron_job(
                    Arc::clone(runtime),
                    Arc::clone(&self.app.entitlements),
                ),
                HostingDeleteCron::cron_job(Arc::clone(runtime)),
            ];

            for job in hosting_jobs {
                match job {
                    Ok(j) => self.cron_jobs.push(j),
                    Err(e) => {
                        tracing::error!(error = ?e, "failed to create cron job");
                    },
                }
            }
        }

        let jobs = [
            StaminaCron::cron_job(),
            Lotto::cron_job::<Self>(),
            HigherLower::cron_job(),
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
            PalworldSaveRefreshCron::cron_job(Arc::clone(&self.palworld)),
            PalworldWarmCron::cron_job(Arc::clone(&self.palworld)),
            EntitlementSweepCron::cron_job(),
            SupportIdleCron::cron_job(),
            SupportIdleCloseCron::cron_job(),
            SupportIdleStaleCron::cron_job(),
            SupportIdleGcCron::cron_job(),
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
        let data = data.read().await;
        data.voice_states.guild_create(guild);
        GuildMembersCache::guild_create(&*data, guild);
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

impl CronJobData for BotState {
    fn jobs(&self) -> &[CronJob] {
        &self.cron_jobs
    }

    fn jobs_mut(&mut self) -> &mut Vec<CronJob> {
        &mut self.cron_jobs
    }
}

impl GuildMembersCache for BotState {
    fn get(&self) -> &DashMap<GuildId, Vec<UserId>> {
        &self.guild_members
    }
}

impl GamblingData for BotState {
    fn game_cache(&self) -> &GameCache {
        &self.gambling_cache
    }
}

impl GoodMorningCache for BotState {
    fn insert(
        &self,
        channel_id: GenericChannelId,
        author: UserId,
        is_good_morning: bool,
    ) -> Option<(UserId, bool)> {
        self.good_morning_cache.insert(channel_id, (author, is_good_morning))
    }
}
