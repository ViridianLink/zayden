use std::sync::Arc;

use jellyfin::runtime::JellyfinRuntime;
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp, Zoned};
use serenity::all::Context;
use tokio::sync::RwLock;
use tracing::error;
use zayden_core::{CronJob, CronJobData};

use crate::party::lifecycle;
use crate::party::row::PartyRow;

pub const PROVISION_LEAD: i64 = 30;

#[must_use]
pub fn job_id(party_id: i64) -> String {
    format!("jellyfin_party_{party_id}")
}

fn schedule(at: &Zoned) -> String {
    format!(
        "0 {} {} {} {} * {}",
        at.minute(),
        at.hour(),
        at.day(),
        at.month(),
        at.year()
    )
}

pub async fn create_jobs<Data: CronJobData>(
    ctx: &Context,
    runtime: &Arc<JellyfinRuntime>,
    party: &PartyRow,
) {
    let party_id = party.id;
    let id = job_id(party_id);
    let start = party.starts_at().to_zoned(TimeZone::UTC);
    let now = Timestamp::now();

    let mut jobs = Vec::with_capacity(6);

    let provision_at = &start - Span::new().minutes(PROVISION_LEAD);
    if provision_at.timestamp() > now {
        let runtime = Arc::clone(runtime);
        if let Some(job) = build(&id, &provision_at, move |ctx, pool| {
            let runtime = Arc::clone(&runtime);
            async move {
                if let Err(e) =
                    lifecycle::provision(&ctx.http, &runtime, &pool, party_id).await
                {
                    error!(error = ?e, party_id, "party provisioning failed");
                }
            }
        }) {
            jobs.push(job);
        }
    }

    let reminders = [
        (Span::new().hours(24), "starts in 24 hours"),
        (Span::new().hours(1), "starts in an hour"),
        (Span::new().minutes(15), "starts in 15 minutes"),
    ];

    for (offset, phrase) in reminders {
        let at = &start - offset;
        if at.timestamp() <= now {
            continue;
        }

        let runtime = Arc::clone(runtime);
        let name = party.item_name.clone();
        if let Some(job) = build(&id, &at, move |ctx, pool| {
            let runtime = Arc::clone(&runtime);
            let content = format!("**{name}** {phrase}.");
            async move {
                lifecycle::announce(&ctx.http, &runtime, &pool, party_id, content)
                    .await;
            }
        }) {
            jobs.push(job);
        }
    }

    if start.timestamp() > now {
        let runtime = Arc::clone(runtime);
        let name = party.item_name.clone();
        if let Some(job) = build(&id, &start, move |ctx, pool| {
            let runtime = Arc::clone(&runtime);
            let content = format!(
                "**{name}** starts now. The host should start playback and open \
                 a SyncPlay group from their client — everyone else, join it \
                 from the cast menu."
            );
            async move {
                lifecycle::announce(&ctx.http, &runtime, &pool, party_id, content)
                    .await;
            }
        }) {
            jobs.push(job);
        }
    }

    let cleanup_at = party.cleanup_after().to_zoned(TimeZone::UTC);
    if cleanup_at.timestamp() > now {
        let runtime = Arc::clone(runtime);
        if let Some(job) = build(&id, &cleanup_at, move |_ctx, pool| {
            let runtime = Arc::clone(&runtime);
            async move {
                if let Err(e) = lifecycle::cleanup(&runtime, &pool, party_id).await {
                    error!(error = ?e, party_id, "party cleanup failed");
                }
            }
        }) {
            jobs.push(job);
        }
    }

    let data = ctx.data::<RwLock<Data>>();
    let mut data = data.write().await;
    data.jobs_mut().retain(|job| job.id != id);
    data.jobs_mut().extend(jobs);
}

pub async fn clear_jobs<Data: CronJobData>(ctx: &Context, party_id: i64) {
    let id = job_id(party_id);

    let data = ctx.data::<RwLock<Data>>();
    let mut data = data.write().await;
    data.jobs_mut().retain(|job| job.id != id);
}

fn build<F, Fut>(id: &str, at: &Zoned, action: F) -> Option<CronJob>
where
    F: Fn(Context, sqlx::PgPool) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    match CronJob::new(id.to_owned(), &schedule(at)) {
        Ok(job) => Some(job.set_action(action)),
        Err(e) => {
            error!(error = ?e, "invalid party cron schedule");
            None
        },
    }
}
