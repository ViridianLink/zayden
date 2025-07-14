use async_trait::async_trait;
use bigdecimal::ToPrimitive;
use gambling::shop::LOTTO_TICKET;
use gambling::{Commands, LottoManager, LottoRow};
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgConnection, PgPool, Postgres};
use zayden_core::SlashCommand;

use crate::modules::gambling::GamblingTable;
use crate::{Error, Result};

pub struct LottoTable;

#[async_trait]
impl LottoManager<Postgres> for LottoTable {
    async fn row(
        conn: &mut PgConnection,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<LottoRow>> {
        let id = id.into();

        sqlx::query_as!(
            LottoRow,
            "SELECT g.id, g.coins, COALESCE(i.quantity, 0) AS quantity FROM gambling g LEFT JOIN gambling_inventory i ON g.id = i.user_id AND i.item_id = $2 WHERE g.id = $1",
            id.get() as i64,
            LOTTO_TICKET.id
        ).fetch_optional(conn).await
    }

    async fn rows(conn: &mut PgConnection) -> sqlx::Result<Vec<LottoRow>> {
        sqlx::query_as!(
            LottoRow,
            "SELECT g.id, g.coins, i.quantity AS quantity FROM gambling g LEFT JOIN gambling_inventory i ON g.id = i.user_id AND i.item_id = $1",
            LOTTO_TICKET.id
        )
        .fetch_all(conn)
        .await
    }

    async fn total_tickets(conn: &mut PgConnection) -> sqlx::Result<i64> {
        sqlx::query_scalar!(
            "SELECT SUM(quantity) FROM gambling_inventory WHERE item_id = $1",
            LOTTO_TICKET.id
        )
        .fetch_one(conn)
        .await
        .map(|x| x.unwrap_or_default())
        .map(|x| x.to_i64().unwrap_or_default())
    }

    async fn delete_tickets(conn: &mut PgConnection) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "DELETE FROM gambling_inventory WHERE item_id = $1",
            LOTTO_TICKET.id
        )
        .execute(conn)
        .await
    }
}

pub struct Lotto;

#[async_trait]
impl SlashCommand<Error, Postgres> for Lotto {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::lotto::<Postgres, GamblingTable, LottoTable>(&ctx.http, interaction, pool)
            .await?;
        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand> {
        Ok(Commands::register_lotto())
    }
}
