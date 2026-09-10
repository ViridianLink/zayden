use std::sync::Arc;

use tracing::error;
use zayden_core::CronJob;

use crate::runtime::JellyfinRuntime;
use crate::stats::rollup;

pub struct JellyfinRollupCron;

impl JellyfinRollupCron {
    pub fn cron_job(
        runtime: Arc<JellyfinRuntime>,
    ) -> Result<CronJob, jiff_cron::error::Error> {
        CronJob::new("jellyfin_playback_rollup", "0 0 4 * * * *").map(|job| {
            job.set_action(move |_ctx, pool| {
                let runtime = Arc::clone(&runtime);

                async move {
                    if let Err(e) = rollup::run(&runtime, &pool).await {
                        error!(error = ?e, "jellyfin: playback rollup failed");
                    }
                }
            })
        })
    }
}
