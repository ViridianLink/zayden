use std::path::Path;
use std::sync::Arc;

use tracing::{error, warn};
use zayden_core::CronJob;

use crate::client::PalworldClient;
use crate::upload::SaveUpload;

pub struct PalworldSaveRefreshCron;

impl PalworldSaveRefreshCron {
    pub fn cron_job(
        client: Arc<PalworldClient>,
    ) -> Result<CronJob, jiff_cron::error::Error> {
        CronJob::new("palworld_save_refresh", "0 */2 * * * * *").map(|job| {
            job.set_action(move |_ctx, _pool| {
                let client = Arc::clone(&client);
                async move { client.warm_player_names().await }
            })
        })
    }
}

pub struct PalworldWarmCron;

impl PalworldWarmCron {
    pub fn cron_job(
        client: Arc<PalworldClient>,
    ) -> Result<CronJob, jiff_cron::error::Error> {
        CronJob::new("palworld_cache_warm", "0 0 */6 * * * *").map(|job| {
            job.set_action(move |_ctx, _pool| {
                let client = Arc::clone(&client);
                async move { client.warm().await }
            })
        })
    }
}

pub struct PalworldUploadSweepCron;

impl PalworldUploadSweepCron {
    pub fn cron_job() -> Result<CronJob, jiff_cron::error::Error> {
        CronJob::new("palworld_upload_sweep", "0 0 * * * * *").map(|job| {
            job.set_action(move |_ctx, pool| async move {
                let paths = match SaveUpload::delete_expired(&pool).await {
                    Ok(paths) => paths,
                    Err(e) => {
                        error!(
                            error = ?e,
                            "palworld: failed to sweep expired uploads"
                        );
                        return;
                    },
                };

                for path in paths {
                    let target = Path::new(&path);
                    let removed = target.parent().map_or_else(
                        || std::fs::remove_file(target),
                        std::fs::remove_dir_all,
                    );
                    if let Err(e) = removed {
                        warn!(
                            error = %e,
                            path,
                            "palworld: failed to remove expired upload"
                        );
                    }
                }
            })
        })
    }
}
