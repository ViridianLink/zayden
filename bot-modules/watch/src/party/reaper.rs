use std::sync::Arc;

use jellyfin::guest;
use jellyfin::runtime::JellyfinRuntime;
use sqlx::PgPool;
use tracing::{error, info, warn};
use zayden_core::CronJob;

use crate::party::lifecycle;
use crate::party::row::PartyRow;

pub struct JellyfinPartyReaperCron;

impl JellyfinPartyReaperCron {
    pub fn cron_job(
        runtime: Arc<JellyfinRuntime>,
    ) -> Result<CronJob, jiff_cron::error::Error> {
        CronJob::new("jellyfin_party_reaper", "0 */10 * * * * *").map(|job| {
            job.set_action(move |_ctx, pool| {
                let runtime = Arc::clone(&runtime);
                async move {
                    sweep_due(&runtime, &pool).await;
                }
            })
        })
    }

    pub fn reconcile_job(
        runtime: Arc<JellyfinRuntime>,
    ) -> Result<CronJob, jiff_cron::error::Error> {
        CronJob::new("jellyfin_party_reconcile", "0 30 * * * * *").map(|job| {
            job.set_action(move |_ctx, pool| {
                let runtime = Arc::clone(&runtime);
                async move {
                    if let Err(e) = reconcile(&runtime, &pool).await {
                        error!(error = ?e, "jellyfin: party reconciliation failed");
                    }
                }
            })
        })
    }
}

async fn sweep_due(runtime: &Arc<JellyfinRuntime>, pool: &PgPool) {
    let due = match PartyRow::due_for_cleanup(pool).await {
        Ok(due) => due,
        Err(e) => {
            error!(error = ?e, "jellyfin: could not list parties due for cleanup");
            return;
        },
    };

    for party in due {
        if let Err(e) = lifecycle::cleanup(runtime, pool, party.id).await {
            error!(error = ?e, party_id = party.id, "party cleanup failed");
        }
    }
}

pub async fn reconcile(
    runtime: &Arc<JellyfinRuntime>,
    pool: &PgPool,
) -> crate::error::Result<()> {
    let managed = guest::list_managed(runtime).await?;
    let mut removed = 0;

    for (jellyfin_user_id, party_id) in managed.guests {
        if is_live(pool, party_id).await? {
            continue;
        }

        match guest::delete_guest(runtime, &jellyfin_user_id).await {
            Ok(()) => removed += 1,
            Err(e) => warn!(error = ?e, party_id, "orphan guest not removed"),
        }
    }

    for (name, party_id) in managed.libraries {
        if is_live(pool, party_id).await? {
            continue;
        }

        match guest::delete_library(runtime, party_id).await {
            Ok(()) => removed += 1,
            Err(e) => warn!(error = ?e, %name, "orphan library not removed"),
        }
    }

    if removed > 0 {
        info!(removed, "jellyfin: reconciled orphaned party objects");
    }

    Ok(())
}

async fn is_live(pool: &PgPool, party_id: i64) -> crate::error::Result<bool> {
    Ok(PartyRow::get(pool, party_id)
        .await?
        .is_some_and(|p| !p.is_cancelled() && p.cleaned_at.is_none()))
}
