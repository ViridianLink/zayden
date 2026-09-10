use std::sync::Arc;

use tracing::error;
use zayden_core::CronJob;

use crate::index;
use crate::runtime::JellyfinRuntime;

pub struct JellyfinIndexRefreshCron;

impl JellyfinIndexRefreshCron {
    pub fn cron_job(
        runtime: Arc<JellyfinRuntime>,
    ) -> Result<CronJob, jiff_cron::error::Error> {
        CronJob::new("jellyfin_index_refresh", "0 15 */6 * * * *").map(|job| {
            job.set_action(move |_ctx, pool| {
                let runtime = Arc::clone(&runtime);

                async move {
                    if let Err(e) = index::refresh(&runtime, &pool).await {
                        error!(error = ?e, "jellyfin: library index refresh failed");
                    }
                }
            })
        })
    }
}
