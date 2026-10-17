use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use deadlock_api::DeadlockApiError;
use jiff::Timestamp;
use serenity::all::Http;
use sqlx::PgPool;
use tracing::{debug, error, info};
use zayden_core::CronJob;

use crate::announce::announce_ready;
use crate::briefing::{self, BriefingOutcome};
use crate::client::DeadlockApi;
use crate::deliver::deliver;
use crate::error::DeadlockError;
use crate::match_poll::{self, SummaryJob};
use crate::patch_watch::{self, WatchOutcome};
use crate::rank_poll::{self, SquadEvent};
use crate::{gc, squad_deliver};

#[derive(Debug, Default)]
pub struct RunGuard(AtomicBool);

#[derive(Debug)]
pub struct RunTicket(Arc<RunGuard>);

impl Drop for RunTicket {
    fn drop(&mut self) {
        self.0.0.store(false, Ordering::Release);
    }
}

impl RunGuard {
    #[must_use]
    pub fn try_start(self: &Arc<Self>) -> Option<RunTicket> {
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
            .then(|| RunTicket(Arc::clone(self)))
    }
}

pub struct DeadlockPatchCron;

impl DeadlockPatchCron {
    pub fn cron_job(
        api: Arc<DeadlockApi>,
        application_id: u64,
    ) -> Result<CronJob, jiff_cron::error::Error> {
        let guard = Arc::new(RunGuard::default());

        CronJob::new("deadlock_patch_watch", "0 */5 * * * * *").map(|job| {
            job.set_action(move |ctx, pool| {
                let api = Arc::clone(&api);
                let guard = Arc::clone(&guard);

                async move {
                    watch(&api, &pool).await;

                    if let Some(ticket) = guard.try_start() {
                        tokio::spawn(announce(
                            ticket,
                            Arc::clone(&ctx.http),
                            pool,
                            application_id.cast_signed(),
                        ));
                    }
                }
            })
        })
    }
}

async fn announce(
    _ticket: RunTicket,
    http: Arc<Http>,
    pool: PgPool,
    application_id: i64,
) {
    if let Err(e) = announce_ready(&http, &pool, application_id).await {
        error!(error = ?e, "deadlock: failed to announce patches");
    }
}

pub struct DeadlockMatchCron;

impl DeadlockMatchCron {
    pub fn cron_job(
        api: Arc<DeadlockApi>,
        application_id: u64,
    ) -> Result<CronJob, jiff_cron::error::Error> {
        let guard = Arc::new(RunGuard::default());

        CronJob::new("deadlock_match_poll", "0 */3 * * * * *").map(|job| {
            job.set_action(move |ctx, pool| {
                let api = Arc::clone(&api);
                let guard = Arc::clone(&guard);

                async move {
                    let Some(ticket) = guard.try_start() else {
                        return;
                    };

                    match match_poll::poll(&api, &pool, Timestamp::now()).await {
                        Ok((outcome, jobs)) => {
                            debug!(?outcome, "deadlock: match poll finished");

                            if !jobs.is_empty() {
                                tokio::spawn(summarize(
                                    ticket,
                                    Arc::clone(&ctx.http),
                                    pool,
                                    api,
                                    application_id.cast_signed(),
                                    jobs,
                                ));
                            }
                        },
                        Err(DeadlockError::Api(
                            e @ DeadlockApiError::Busy { .. },
                        )) => {
                            debug!(error = %e, "deadlock: match poll deferred, deadlock-api is busy");
                        },
                        Err(e) => error!(error = ?e, "deadlock: match poll failed"),
                    }
                }
            })
        })
    }
}

async fn summarize(
    _ticket: RunTicket,
    http: Arc<Http>,
    pool: PgPool,
    api: Arc<DeadlockApi>,
    application_id: i64,
    jobs: Vec<SummaryJob>,
) {
    match deliver(&http, &pool, &api, application_id, jobs).await {
        Ok(outcome) => debug!(?outcome, "deadlock: match summaries delivered"),
        Err(e) => error!(error = ?e, "deadlock: failed to deliver match summaries"),
    }
}

pub struct DeadlockRankCron;

impl DeadlockRankCron {
    pub fn cron_job(
        api: Arc<DeadlockApi>,
        application_id: u64,
    ) -> Result<CronJob, jiff_cron::error::Error> {
        let guard = Arc::new(RunGuard::default());

        CronJob::new("deadlock_rank_poll", "0 7,37 * * * * *").map(|job| {
            job.set_action(move |ctx, pool| {
                let api = Arc::clone(&api);
                let guard = Arc::clone(&guard);

                async move {
                    let Some(ticket) = guard.try_start() else {
                        return;
                    };

                    match rank_poll::poll(&api, &pool, Timestamp::now()).await {
                        Ok((outcome, events)) => {
                            debug!(?outcome, "deadlock: rank poll finished");

                            if !events.is_empty() {
                                tokio::spawn(announce_squad(
                                    ticket,
                                    Arc::clone(&ctx.http),
                                    pool,
                                    api,
                                    application_id.cast_signed(),
                                    events,
                                ));
                            }
                        },
                        Err(DeadlockError::Api(
                            e @ DeadlockApiError::Busy { .. },
                        )) => {
                            debug!(error = %e, "deadlock: rank poll deferred, deadlock-api is busy");
                        },
                        Err(e) => error!(error = ?e, "deadlock: rank poll failed"),
                    }
                }
            })
        })
    }
}

async fn announce_squad(
    _ticket: RunTicket,
    http: Arc<Http>,
    pool: PgPool,
    api: Arc<DeadlockApi>,
    application_id: i64,
    events: Vec<SquadEvent>,
) {
    match squad_deliver::deliver(&http, &pool, &api, application_id, events).await {
        Ok(posts) => debug!(posts, "deadlock: squad events delivered"),
        Err(e) => error!(error = ?e, "deadlock: failed to deliver squad events"),
    }
}

pub struct DeadlockLeaderboardCron;

impl DeadlockLeaderboardCron {
    pub fn cron_job(
        api: Arc<DeadlockApi>,
        application_id: u64,
    ) -> Result<CronJob, jiff_cron::error::Error> {
        let guard = Arc::new(RunGuard::default());

        CronJob::new("deadlock_leaderboard", "0 0 18 * * Sun *").map(|job| {
            job.set_action(move |ctx, pool| {
                let api = Arc::clone(&api);
                let guard = Arc::clone(&guard);

                async move {
                    if let Some(ticket) = guard.try_start() {
                        tokio::spawn(post_leaderboards(
                            ticket,
                            Arc::clone(&ctx.http),
                            pool,
                            api,
                            application_id.cast_signed(),
                        ));
                    }
                }
            })
        })
    }
}

async fn post_leaderboards(
    _ticket: RunTicket,
    http: Arc<Http>,
    pool: PgPool,
    api: Arc<DeadlockApi>,
    application_id: i64,
) {
    match squad_deliver::post_leaderboards(&http, &pool, &api, application_id).await
    {
        Ok(posts) => debug!(posts, "deadlock: squad leaderboards posted"),
        Err(e) => error!(error = ?e, "deadlock: failed to post squad leaderboards"),
    }
}

pub struct DeadlockBriefingCron;

impl DeadlockBriefingCron {
    pub fn cron_job() -> Result<CronJob, jiff_cron::error::Error> {
        let guard = Arc::new(RunGuard::default());

        CronJob::new("deadlock_patch_briefing", "30 */10 * * * * *").map(|job| {
            job.set_action(move |ctx, pool| {
                let guard = Arc::clone(&guard);

                async move {
                    if let Some(ticket) = guard.try_start() {
                        tokio::spawn(brief(ticket, Arc::clone(&ctx.http), pool));
                    }
                }
            })
        })
    }
}

async fn brief(_ticket: RunTicket, http: Arc<Http>, pool: PgPool) {
    match briefing::run(&http, &pool, Timestamp::now()).await {
        Ok(outcome) if outcome == BriefingOutcome::default() => {
            debug!("deadlock: no patch briefings to send");
        },
        Ok(outcome) => info!(?outcome, "deadlock: patch briefings sent"),
        Err(e) => error!(error = ?e, "deadlock: patch briefing failed"),
    }
}

pub struct DeadlockCacheGcCron;

impl DeadlockCacheGcCron {
    pub fn cron_job() -> Result<CronJob, jiff_cron::error::Error> {
        CronJob::new("deadlock_cache_gc", "0 15 4 * * * *").map(|job| {
            job.set_action(|_ctx, pool| async move {
                match gc::run(&pool, Timestamp::now()).await {
                    Ok(outcome) => info!(?outcome, "deadlock: cache gc finished"),
                    Err(e) => error!(error = ?e, "deadlock: cache gc failed"),
                }
            })
        })
    }
}

async fn watch(api: &DeadlockApi, pool: &PgPool) {
    match patch_watch::run(api, pool, Timestamp::now()).await {
        Ok(outcome) if outcome == WatchOutcome::default() => {
            debug!("deadlock: patch watch found nothing new");
        },
        Ok(outcome) => info!(?outcome, "deadlock: patch watch made progress"),
        Err(DeadlockError::Api(e @ DeadlockApiError::Busy { .. })) => {
            debug!(error = %e, "deadlock: patch watch deferred, deadlock-api is busy");
        },
        Err(e) => error!(error = ?e, "deadlock: patch watch failed"),
    }
}
