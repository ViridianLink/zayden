use serenity::all::{ForumTagId, Http};
use tracing::{debug, error};
use zayden_core::CronJob;

use crate::batch;
use crate::idle::act;
use crate::idle::close::claim_due as claim_due_close;
use crate::idle::stale::{StaleTarget, claim_cleared, claim_due as claim_due_stale};
use crate::idle::sweep::{claim_due, gc};

const NUDGE_BATCH: i64 = 40;
const CLOSE_BATCH: i64 = 40;
const STALE_BATCH: i64 = 40;

pub struct SupportIdleCron;

impl SupportIdleCron {
    pub fn cron_job() -> Result<CronJob, jiff_cron::error::Error> {
        CronJob::new("support_idle_sweep", "0 */5 * * * * *").map(|job| {
            job.set_action(move |ctx, pool| async move {
                let due = match claim_due(&pool, NUDGE_BATCH).await {
                    Ok(due) => due,
                    Err(e) => {
                        error!(error = ?e, "support idle sweep failed");
                        return;
                    },
                };

                let http = &ctx.http;
                let pool = &pool;

                batch::run(due, move |row| async move {
                    act::nudge(http, pool, &row).await;
                })
                .await;
            })
        })
    }
}

pub struct SupportIdleCloseCron;

impl SupportIdleCloseCron {
    pub fn cron_job() -> Result<CronJob, jiff_cron::error::Error> {
        CronJob::new("support_idle_close", "30 */5 * * * * *").map(|job| {
            job.set_action(move |ctx, pool| async move {
                let due = match claim_due_close(&pool, CLOSE_BATCH).await {
                    Ok(due) => due,
                    Err(e) => {
                        error!(error = ?e, "support idle close sweep failed");
                        return;
                    },
                };

                let http = &ctx.http;
                let pool = &pool;

                let requests = due
                    .iter()
                    .map(|row| {
                        (
                            row.guild_id,
                            row.guild(),
                            row.support_channel(),
                            row.closed_tag(),
                        )
                    })
                    .collect::<Vec<_>>();

                let tags = act::resolve_tags(http, &requests).await;
                let tags = &tags;

                batch::run(due, move |row| async move {
                    act::close(http, pool, &row, act::tag_for(tags, row.guild_id))
                        .await;
                })
                .await;
            })
        })
    }
}

pub struct SupportIdleStaleCron;

impl SupportIdleStaleCron {
    pub fn cron_job() -> Result<CronJob, jiff_cron::error::Error> {
        CronJob::new("support_idle_stale", "15 */5 * * * * *").map(|job| {
            job.set_action(move |ctx, pool| async move {
                let http = &ctx.http;
                let pool = &pool;

                match claim_due_stale(pool, STALE_BATCH).await {
                    Ok(due) => {
                        let tags = tags(http, &due).await;
                        let tags = &tags;

                        batch::run(due, move |row| async move {
                            act::stale(
                                http,
                                pool,
                                &row,
                                act::tag_for(tags, row.guild_id),
                            )
                            .await;
                        })
                        .await;
                    },
                    Err(e) => error!(error = ?e, "support stale sweep failed"),
                }

                match claim_cleared(pool, STALE_BATCH).await {
                    Ok(cleared) => {
                        let tags = tags(http, &cleared).await;
                        let tags = &tags;

                        batch::run(cleared, move |row| async move {
                            act::unstale(
                                http,
                                pool,
                                &row,
                                act::tag_for(tags, row.guild_id),
                            )
                            .await;
                        })
                        .await;
                    },
                    Err(e) => error!(error = ?e, "support stale clear sweep failed"),
                }
            })
        })
    }
}

pub struct SupportIdleGcCron;

impl SupportIdleGcCron {
    pub fn cron_job() -> Result<CronJob, jiff_cron::error::Error> {
        CronJob::new("support_idle_gc", "0 0 4 * * * *").map(|job| {
            job.set_action(move |_ctx, pool| async move {
                match gc(&pool).await {
                    Ok(n) if n > 0 => debug!(dropped = n, "support idle gc"),
                    Ok(_) => {},
                    Err(e) => error!(error = ?e, "support idle gc failed"),
                }
            })
        })
    }
}

async fn tags(http: &Http, rows: &[StaleTarget]) -> Vec<(i64, Option<ForumTagId>)> {
    let requests = rows
        .iter()
        .map(|row| {
            (row.guild_id, row.guild(), row.support_channel(), row.stale_tag())
        })
        .collect::<Vec<_>>();

    act::resolve_tags(http, &requests).await
}
