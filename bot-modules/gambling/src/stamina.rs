use jiff_cron;
use sqlx::postgres::PgQueryResult;
use sqlx::{PgPool, Postgres};
use zayden_core::CronJob;

pub const MAX_STAMINA: i32 = 3;

pub struct StaminaManager;

impl StaminaManager {
    pub async fn update(pool: &PgPool) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "UPDATE gambling SET stamina = stamina + 1 WHERE stamina < $1",
            MAX_STAMINA
        )
        .execute(pool)
        .await
    }
}

pub struct StaminaCron;

impl StaminaCron {
    pub fn cron_job() -> Result<CronJob<Postgres>, jiff_cron::error::Error> {
        Ok(CronJob::new("stamina", "0 */10 * * * * *")?.set_action(
            |_ctx, pool| async move {
                if let Err(e) = StaminaManager::update(&pool).await {
                    tracing::error!(error = ?e, "stamina cron update failed");
                }
            },
        ))
    }
}
