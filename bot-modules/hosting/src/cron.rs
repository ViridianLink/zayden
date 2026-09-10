use std::sync::Arc;

use jiff_cron::error::Error as CronError;
use serenity::all::{CreateMessage, Http, UserId};
use tracing::{error, warn};
use zayden_app::entitlement::EntitlementService;
use zayden_core::cron::CronJob;

use crate::sweep::{self, Claimed};
use crate::{HostingRuntime, pricing, store};

const BATCH: i64 = 50;

pub struct HostingReminderCron;

impl HostingReminderCron {
    pub fn cron_job(runtime: Arc<HostingRuntime>) -> Result<CronJob, CronError> {
        CronJob::new("hosting_reminder", "0 0 * * * * *").map(|job| {
            job.set_action(move |ctx, pool| {
                let runtime = Arc::clone(&runtime);
                async move {
                    let days = clamp_days(runtime.config.reminder_days);

                    let due = match sweep::claim_reminders(&pool, days, BATCH).await
                    {
                        Ok(due) => due,
                        Err(e) => {
                            error!(error = %e, "hosting reminder sweep failed");
                            return;
                        },
                    };

                    for row in due {
                        remind(&ctx.http, &runtime, &row).await;
                    }
                }
            })
        })
    }
}

pub struct HostingExpireCron;

impl HostingExpireCron {
    pub fn cron_job(
        runtime: Arc<HostingRuntime>,
        entitlements: Arc<EntitlementService>,
    ) -> Result<CronJob, CronError> {
        CronJob::new("hosting_expire", "0 */5 * * * * *").map(|job| {
            job.set_action(move |ctx, pool| {
                let runtime = Arc::clone(&runtime);
                let entitlements = Arc::clone(&entitlements);
                async move {
                    let grace = clamp_days(runtime.config.grace_days);

                    match sweep::convert_included_trials(&pool, BATCH).await {
                        Ok(rows) => {
                            for row in rows {
                                notify_converted(&ctx.http, &row).await;
                            }
                        },
                        Err(e) => {
                            error!(error = %e, "hosting trial conversion failed");
                        },
                    }

                    match sweep::claim_expired(&pool, grace, BATCH).await {
                        Ok(rows) => {
                            for row in rows {
                                suspend(&ctx.http, &runtime, &row).await;
                            }
                        },
                        Err(e) => error!(error = %e, "hosting expire sweep failed"),
                    }

                    revalidate(&ctx.http, &runtime, &entitlements, &pool, grace)
                        .await;
                }
            })
        })
    }
}

pub struct HostingDeleteCron;

impl HostingDeleteCron {
    pub fn cron_job(runtime: Arc<HostingRuntime>) -> Result<CronJob, CronError> {
        CronJob::new("hosting_delete", "0 30 * * * * *").map(|job| {
            job.set_action(move |ctx, pool| {
                let runtime = Arc::clone(&runtime);
                async move {
                    let due = match sweep::claim_deletions(&pool, BATCH).await {
                        Ok(due) => due,
                        Err(e) => {
                            error!(error = %e, "hosting delete sweep failed");
                            return;
                        },
                    };

                    for row in due {
                        delete(&ctx.http, &runtime, &pool, &row).await;
                    }
                }
            })
        })
    }
}

async fn remind(http: &Http, runtime: &HostingRuntime, row: &Claimed) {
    let when = row
        .expires_at()
        .map_or_else(|| "soon".to_owned(), |t| format!("<t:{}:R>", t.as_second()));

    let price = pricing::format_price(i64::from(row.price_cents));
    let kofi = &runtime.config.kofi_url;

    let content = if row.price_cents == 0 {
        format!(
            "Your **{}** server renews {when} against your subscription. \
             Nothing to do — just keep the subscription active.",
            row.game_key
        )
    } else {
        format!(
            "Your **{}** server expires {when}.\nRenew for **{price}/month** at \
             {kofi} and put `{}` in the message so I know which server it is.",
            row.game_key, row.claim_code
        )
    };

    dm(http, row.owner(), &content).await;
}

async fn notify_converted(http: &Http, row: &Claimed) {
    let content = format!(
        "Your **{}** trial has ended, and your subscription covers this plan — \
         the server stays up at no extra cost.",
        row.game_key
    );

    dm(http, row.owner(), &content).await;
}

async fn suspend(http: &Http, runtime: &HostingRuntime, row: &Claimed) {
    if let Some(server_id) = row.pelican_server_id
        && let Err(e) = runtime.pelican.suspend(server_id).await
    {
        warn!(error = %e, row = row.id, "hosting: panel suspend failed");
    }

    let price = pricing::format_price(i64::from(row.price_cents));
    let kofi = &runtime.config.kofi_url;

    // A trial that was never paid for gets no grace, so it is about to go —
    // promising N days here would be a straight lie.
    let content = if row.has_paid {
        format!(
            "Your **{}** server has been suspended — its paid window ran out.\n\
             It will be deleted in {} days. Renew for **{price}/month** at \
             {kofi} with `{}` in the message and it comes straight back.",
            row.game_key, runtime.config.grace_days, row.claim_code
        )
    } else {
        format!(
            "Your **{}** trial has ended and the server is suspended. It will \
             be deleted shortly. Subscribe for **{price}/month** at {kofi} with \
             `{}` in the message to keep it — do that before it is deleted and \
             the world carries over.",
            row.game_key, row.claim_code
        )
    };

    dm(http, row.owner(), &content).await;
}

async fn delete(
    http: &Http,
    runtime: &HostingRuntime,
    pool: &sqlx::PgPool,
    row: &Claimed,
) {
    if let Some(server_id) = row.pelican_server_id
        && let Err(e) = runtime.pelican.delete_server(server_id).await
    {
        // Leaving the row claimed means the lease expires and this retries,
        // rather than the row being marked deleted while the server lives on.
        warn!(error = %e, row = row.id, "hosting: panel delete failed; will retry");
        return;
    }

    if let Err(e) = store::mark_deleted(pool, row.id).await {
        error!(error = %e, row = row.id, "hosting: could not mark row deleted");
        return;
    }

    let content = format!(
        "Your **{}** server has been deleted after its grace period. Its files \
         are gone; `/server host` will set up a fresh one whenever you want.",
        row.game_key
    );

    dm(http, row.owner(), &content).await;
}

async fn revalidate(
    http: &Http,
    runtime: &HostingRuntime,
    entitlements: &EntitlementService,
    pool: &sqlx::PgPool,
    grace: i32,
) {
    let rows = match sweep::entitlement_backed(pool, BATCH).await {
        Ok(rows) => rows,
        Err(e) => {
            error!(error = %e, "hosting entitlement revalidation failed");
            return;
        },
    };

    for row in rows {
        let Ok(plan) = row.plan() else {
            warn!(row = row.id, plan = %row.plan, "hosting: unknown plan on row");
            continue;
        };

        let tier = entitlements.user_tier(row.owner().get()).await;
        if pricing::is_included(plan, tier) {
            continue;
        }

        if let Err(e) = sweep::suspend(pool, row.id, grace).await {
            error!(error = %e, row = row.id, "hosting: suspend failed");
            continue;
        }

        suspend(http, runtime, &row).await;
    }
}

async fn dm(http: &Http, user: UserId, content: &str) {
    if let Err(e) =
        user.direct_message(http, CreateMessage::new().content(content)).await
    {
        warn!(error = %e, %user, "hosting: could not DM the owner");
    }
}

fn clamp_days(days: i64) -> i32 {
    i32::try_from(days.clamp(0, 365)).unwrap_or(0)
}
