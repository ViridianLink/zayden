use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use deadlock_api::DeadlockApiError;
use jiff::Timestamp;
use serenity::all::Http;
use sqlx::PgPool;
use tracing::{debug, error, info};
use zayden_core::CronJob;

use crate::announce::announce_ready;
use crate::client::DeadlockApi;
use crate::error::DeadlockError;
use crate::patch_watch::{self, WatchOutcome};

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
