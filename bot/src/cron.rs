use std::cmp::Ordering;
use std::sync::Arc;
use std::time::Duration;

use futures::future;
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp, Zoned};
use serenity::all::Context;
use sqlx::{PgPool, Postgres};
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{debug, error, info};
use zayden_core::{ActionFn, CronJob, CronJobData};

use crate::{BotState, Result};

pub struct EntitlementSweepCron;

impl EntitlementSweepCron {
    pub fn cron_job()
    -> std::result::Result<CronJob<Postgres>, jiff_cron::error::Error> {
        CronJob::new("entitlement_expiry_sweep", "0 0 * * * * *").map(|job| {
            job.set_action(|ctx, _pool| async move {
                let entitlements = {
                    let data = ctx.data::<RwLock<BotState>>();
                    let state = data.read().await;
                    Arc::clone(&state.app.entitlements)
                };
                match entitlements.refresh_expired_cache_rows().await {
                    Ok(n) if n > 0 => {
                        info!(demoted = n, "entitlement expiry sweep completed");
                    },
                    Ok(_) => {},
                    Err(e) => {
                        error!(error = ?e, "entitlement expiry sweep failed");
                    },
                }
            })
        })
    }
}

pub async fn start_cron_jobs(ctx: Context, pool: PgPool) {
    if let Err(e) = run_cron_jobs_loop(ctx, pool).await {
        error!("Error starting cron jobs: {e:?}");
    }
}

async fn run_cron_jobs_loop(ctx: Context, pool: PgPool) -> Result<()> {
    loop {
        let pending_jobs = pending_jobs(&ctx).await;

        let sleep_duration = match pending_jobs.first() {
            Some((target_wakeup_time, _)) => {
                debug!("Next Job: {target_wakeup_time:?}");

                let now = Timestamp::now().to_zoned(TimeZone::UTC);
                if *target_wakeup_time > now {
                    target_wakeup_time.duration_since(&now)
                } else {
                    SignedDuration::new(0, 0)
                }
            },
            None => SignedDuration::new(60, 0),
        };

        if sleep_duration > SignedDuration::new(1, 0) {
            let std_duration = sleep_duration.try_into()?;
            sleep(std_duration).await;
        }

        if !pending_jobs.is_empty() {
            let futures_iter = pending_jobs
                .into_iter()
                .map(|(_, action)| (action)(ctx.clone(), pool.clone()));

            future::join_all(futures_iter).await;
        }

        sleep(Duration::from_secs(5)).await;
    }
}

async fn pending_jobs(ctx: &Context) -> Vec<(Zoned, ActionFn<Postgres>)> {
    let mut pending_jobs: Vec<(Zoned, ActionFn<Postgres>)> = Vec::new();

    let data = ctx.data::<RwLock<BotState>>();

    let now = Timestamp::now().to_zoned(TimeZone::UTC);

    let jobs: Vec<(Zoned, ActionFn<Postgres>)> = {
        let mut data = data.write().await;
        // TODO(M9-correctness): verify retain predicate — `upcoming().next()`
        // already returns a future time, so `t > now` may be redundant, and
        // `includes(t)` on the same schedule's own upcoming result should
        // always be true.
        data.jobs_mut().retain(|job| {
            job.schedule
                .upcoming(TimeZone::UTC)
                .next()
                .is_some_and(|t| t > now && job.schedule.includes(t))
        });
        data.jobs()
            .iter()
            .filter_map(|job| {
                job.schedule
                    .upcoming(TimeZone::UTC)
                    .next()
                    .filter(|t| *t > now && job.schedule.includes(t.clone()))
                    .map(|run_time| (run_time, Arc::clone(&job.action_fn)))
            })
            .collect()
    };

    for (run_time, action_fn) in jobs {
        let cmp = pending_jobs.first().map(|(t, _)| run_time.cmp(t));
        match cmp {
            Some(Ordering::Less) | None => {
                pending_jobs = vec![(run_time, action_fn)];
            },
            Some(Ordering::Equal) => {
                pending_jobs.push((run_time, action_fn));
            },
            Some(Ordering::Greater) => {},
        }
    }

    pending_jobs
}
