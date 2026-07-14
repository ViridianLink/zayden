use sqlx::Postgres;
use tracing::{error, warn};
use zayden_core::CronJob;

use crate::upload::SaveUpload;

pub struct PalworldUploadSweepCron;

impl PalworldUploadSweepCron {
    pub fn cron_job() -> Result<CronJob<Postgres>, jiff_cron::error::Error> {
        CronJob::new("palworld_upload_sweep", "0 0 * * * * *").map(|job| {
            job.set_action(move |_ctx, pool| async move {
                match SaveUpload::delete_expired(&pool).await {
                    Ok(paths) => {
                        for path in paths {
                            if let Err(e) = std::fs::remove_file(&path) {
                                warn!(
                                    error = %e,
                                    path,
                                    "palworld: failed to remove expired upload file"
                                );
                            }
                        }
                    },
                    Err(e) => error!(
                        error = ?e,
                        "palworld: failed to sweep expired uploads"
                    ),
                }
            })
        })
    }
}
