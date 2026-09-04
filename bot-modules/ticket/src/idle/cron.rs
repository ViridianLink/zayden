use serenity::all::{Http, HttpError, JsonErrorCode};
use sqlx::PgPool;
use tracing::{debug, error, warn};
use zayden_core::CronJob;

use crate::idle::activity::ThreadActivity;
use crate::idle::reminder;
use crate::idle::sweep::{DueNudge, claim_due, gc};

const NUDGE_BATCH: i64 = 40;

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

                for row in due {
                    send(&ctx.http, &pool, &row).await;
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

async fn send(http: &Http, pool: &PgPool, row: &DueNudge) {
    let Some(reminder) =
        reminder(row.ball(), row.op(), row.helper(), &row.support_roles())
    else {
        debug!(
            thread_id = row.thread_id,
            "nobody to remind; the guild has no support roles",
        );
        return;
    };

    let sent =
        row.thread().widen().send_message(http, reminder.message(row.since())).await;

    let Err(e) = sent else {
        return;
    };

    // The thread can be deleted, or the bot locked out of it, between the claim
    // and the send.
    match code(&e) {
        Some(&JsonErrorCode::UnknownChannel) => {
            if let Err(e) = ThreadActivity::delete(pool, row.thread()).await {
                warn!(error = ?e, thread_id = row.thread_id, "could not drop activity row");
            }
        },
        Some(&JsonErrorCode::MissingAccess | &JsonErrorCode::ThreadLocked) => {
            debug!(thread_id = row.thread_id, "no access to remind; pausing");

            if let Err(e) = ThreadActivity::pause(pool, row.thread()).await {
                warn!(error = ?e, thread_id = row.thread_id, "could not pause activity row");
            }
        },
        _ => warn!(error = ?e, thread_id = row.thread_id, "idle reminder not sent"),
    }
}

fn code(e: &serenity::Error) -> Option<&JsonErrorCode> {
    let serenity::Error::Http(HttpError::UnsuccessfulRequest(resp)) = e else {
        return None;
    };

    Some(&resp.error.code)
}
