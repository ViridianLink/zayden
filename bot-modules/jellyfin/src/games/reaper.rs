use sqlx::PgPool;
use tracing::{error, info};
use zayden_core::CronJob;

use crate::games::round::RoundRow;

pub struct JellyfinGameRoundReaperCron;

impl JellyfinGameRoundReaperCron {
    pub fn cron_job() -> Result<CronJob, jiff_cron::error::Error> {
        CronJob::new("jellyfin_game_round_reaper", "0 15 4 * * * *").map(|job| {
            job.set_action(|_ctx, pool| async move { sweep(&pool).await })
        })
    }
}

async fn sweep(pool: &PgPool) {
    match RoundRow::sweep_finished(pool).await {
        Ok(0) => {},
        Ok(removed) => info!(removed, "jellyfin: swept finished game rounds"),
        Err(e) => error!(error = ?e, "jellyfin: could not sweep game rounds"),
    }
}
