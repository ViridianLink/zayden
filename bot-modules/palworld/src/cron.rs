use std::path::Path;

use tracing::{error, warn};
use zayden_core::CronJob;

use crate::upload::SaveUpload;

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
