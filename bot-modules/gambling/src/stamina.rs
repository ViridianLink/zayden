use async_trait::async_trait;
use jiff_cron;
use sqlx::{Database, Pool};
use zayden_core::CronJob;

#[async_trait]
pub trait StaminaManager<Db: Database> {
    async fn update(pool: &Pool<Db>) -> sqlx::Result<Db::QueryResult>;
}

pub struct StaminaCron;

impl StaminaCron {
    pub fn cron_job<Db: Database, Manager: StaminaManager<Db>>()
    -> Result<CronJob<Db>, jiff_cron::error::Error> {
        Ok(CronJob::new("stamina", "0 */10 * * * * *")?.set_action(
            |_ctx, pool| async move {
                if let Err(e) = Manager::update(&pool).await {
                    tracing::error!(error = ?e, "stamina cron update failed");
                }
            },
        ))
    }
}
