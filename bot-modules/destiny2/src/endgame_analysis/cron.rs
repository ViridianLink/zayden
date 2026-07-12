use std::sync::Arc;

use bungie_api::BungieClient;
use jiff_cron;
use sqlx::Postgres;
use zayden_core::CronJob;

use crate::compendium;
use crate::endgame_analysis::sheet::EndgameAnalysisSheet;

pub struct EndgameAnalysisSheetCron;

impl EndgameAnalysisSheetCron {
    pub fn cron_job(
        bungie_client: Arc<BungieClient>,
        google_api_key: String,
    ) -> Result<CronJob<Postgres>, jiff_cron::error::Error> {
        CronJob::new("endgame_analysis_sheet_weekly", "0 0 0 * * Mon *").map(|job| {
            job.set_action(move |_ctx, pool| {
                let bungie_client = Arc::clone(&bungie_client);
                let google_api_key = google_api_key.clone();
                async move {
                    let manifest =
                        EndgameAnalysisSheet::item_manifest(&bungie_client).await;
                    if let Err(e) = EndgameAnalysisSheet::update(
                        &pool,
                        &manifest,
                        &google_api_key,
                    )
                    .await
                    {
                        tracing::error!(
                            error = ?e,
                            "endgame_analysis sheet update failed"
                        );
                    }
                    if let Err(e) = compendium::update(&pool, &google_api_key).await
                    {
                        tracing::error!(
                            error = ?e,
                            "destiny2 compendium update failed"
                        );
                    }
                }
            })
        })
    }
}
