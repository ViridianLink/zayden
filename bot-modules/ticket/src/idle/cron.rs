use serenity::all::{
    EditThread,
    ForumTagId,
    Http,
    HttpError,
    JsonErrorCode,
    ThreadId,
};
use sqlx::PgPool;
use tracing::{debug, error, warn};
use zayden_core::CronJob;

use crate::idle::activity::ThreadActivity;
use crate::idle::close::{DueClose, claim_due as claim_due_close};
use crate::idle::notice::Notice;
use crate::idle::sweep::{DueNudge, claim_due, gc};
use crate::idle::{batch, reminder};
use crate::state;

const NUDGE_BATCH: i64 = 40;
const CLOSE_BATCH: i64 = 40;

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
                    send(http, pool, &row).await;
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
                let tags = resolve_tags(http, &due).await;
                let tags = &tags;

                batch::run(due, move |row| async move {
                    let tag = tags
                        .iter()
                        .find(|(guild_id, _)| *guild_id == row.guild_id)
                        .and_then(|(_, tag)| *tag);

                    close(http, pool, &row, tag).await;
                })
                .await;
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

async fn resolve_tags(
    http: &Http,
    due: &[DueClose],
) -> Vec<(i64, Option<ForumTagId>)> {
    let mut tags: Vec<(i64, Option<ForumTagId>)> = Vec::new();

    for row in due {
        if tags.iter().any(|(guild_id, _)| *guild_id == row.guild_id) {
            continue;
        }

        let tag = match row.support_channel() {
            Some(channel_id) => {
                state::usable_tag(http, row.guild(), channel_id, row.closed_tag())
                    .await
                    .unwrap_or_default()
            },
            None => None,
        };

        tags.push((row.guild_id, tag));
    }

    tags
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
    triage(pool, row.thread(), row.thread_id, &e, "idle reminder not sent").await;
}

async fn close(http: &Http, pool: &PgPool, row: &DueClose, tag: Option<ForumTagId>) {
    let notice = Notice::new(row.op(), row.since());

    if let Err(e) = row.thread().widen().send_message(http, notice.message()).await {
        triage(pool, row.thread(), row.thread_id, &e, "close notice not sent").await;
        return;
    }

    let edit =
        match state::marking(http, row.guild(), row.thread(), tag, state::CLOSED)
            .await
        {
            Ok(edit) => edit.unwrap_or_default(),
            Err(e) => {
                warn!(
                    error = ?e,
                    thread_id = row.thread_id,
                    "could not read thread to tag it closed; archiving anyway",
                );
                EditThread::new()
            },
        };

    if let Err(e) = row.thread().edit(http, edit.archived(true)).await {
        triage(pool, row.thread(), row.thread_id, &e, "thread not archived").await;
    }
}

async fn triage(
    pool: &PgPool,
    thread: ThreadId,
    thread_id: i64,
    e: &serenity::Error,
    context: &str,
) {
    match code(e) {
        Some(&JsonErrorCode::UnknownChannel) => {
            if let Err(e) = ThreadActivity::delete(pool, thread).await {
                warn!(error = ?e, thread_id, "could not drop activity row");
            }
        },
        Some(&JsonErrorCode::MissingAccess | &JsonErrorCode::ThreadLocked) => {
            debug!(thread_id, context, "no access to the thread; pausing");

            if let Err(e) = ThreadActivity::pause(pool, thread).await {
                warn!(error = ?e, thread_id, "could not pause activity row");
            }
        },
        _ => warn!(error = ?e, thread_id, "{context}"),
    }
}

fn code(e: &serenity::Error) -> Option<&JsonErrorCode> {
    let serenity::Error::Http(HttpError::UnsuccessfulRequest(resp)) = e else {
        return None;
    };

    Some(&resp.error.code)
}
