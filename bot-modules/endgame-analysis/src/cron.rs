use std::sync::Arc;

use bungie_api::BungieClient;
use jiff_cron;
use sqlx::Database;
use zayden_core::CronJob;

use crate::endgame_analysis::EndgameAnalysisSheet;

pub struct EndgameAnalysisSheetCron;

impl EndgameAnalysisSheetCron {
    pub fn cron_job<Db: Database>(
        bungie_client: Arc<BungieClient>,
        google_api_key: String,
    ) -> Result<CronJob<Db>, jiff_cron::error::Error> {
        CronJob::new("endgame_analysis_sheet_weekly", "0 0 0 * * Mon *").map(|job| {
            job.set_action(move |_ctx, _pool| {
                let bungie_client = Arc::clone(&bungie_client);
                let google_api_key = google_api_key.clone();
                async move {
                    let manifest =
                        EndgameAnalysisSheet::item_manifest(&bungie_client).await;
                    if let Err(e) =
                        EndgameAnalysisSheet::update(&manifest, &google_api_key)
                            .await
                    {
                        tracing::error!(
                            error = ?e,
                            "endgame_analysis sheet update failed"
                        );
                    }
                    destiny2::compendium::update(&google_api_key).await;
                }
            })
        })
    }
}
