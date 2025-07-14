use async_trait::async_trait;
use gambling::StaminaManager;
use sqlx::{PgPool, Postgres, postgres::PgQueryResult};

pub const MAX_STAMINA: i32 = 3;

pub struct StaminaTable;

#[async_trait]
impl StaminaManager<Postgres> for StaminaTable {
    async fn update(pool: &PgPool) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "UPDATE gambling SET stamina = LEAST(stamina + 1, $1)",
            MAX_STAMINA
        )
        .execute(pool)
        .await
    }
}
