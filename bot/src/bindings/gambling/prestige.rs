use std::borrow::Cow;

use async_trait::async_trait;
use gambling::commands::inventory::{InventoryManager, InventoryRow};
use gambling::commands::prestige::{PrestigeManager, PrestigeRow};
use gambling::shop::LOTTO_TICKET;
use gambling::{Commands, GamblingItem, GamblingItems};
use serenity::all::{CreateCommand, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgConnection, PgPool, Postgres};
use tracing::warn;
use zayden_core::as_i64;
use zayden_core::ctx::{ComponentCtx, InvocationCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleCommand, ModuleComponent};
use zayden_core::scope::IdMatch;

use super::stamina::MAX_STAMINA;

pub struct PrestigeTable;

#[async_trait]
impl PrestigeManager<Postgres> for PrestigeTable {
    async fn miners(
        pool: &PgPool,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<i64>> {
        let id = id.into();

        sqlx::query_scalar!(
            "SELECT miners FROM gambling_mine WHERE user_id = $1;",
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    async fn row(
        pool: &PgPool,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<PrestigeRow>> {
        let id = id.into();

        sqlx::query_file_as!(
            PrestigeRow,
            "./sql/gambling/PrestigeManager/row.sql",
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }

    async fn lotto(
        pool: &PgPool,
        tickets: i64,
        zayden_id: u64,
    ) -> sqlx::Result<PgQueryResult> {
        sqlx::query_file!(
            "./sql/gambling/PrestigeManager/lotto.sql",
            as_i64(zayden_id),
            LOTTO_TICKET.id,
            tickets,
        )
        .execute(pool)
        .await
    }

    async fn save(
        pool: &PgPool,
        row: PrestigeRow,
        expected_prestige: i64,
    ) -> sqlx::Result<bool> {
        let mut tx = pool.begin().await?;

        let mine = sqlx::query!(
            "UPDATE gambling_mine SET
                miners = $2,
                mines = $3,
                land = $4,
                countries = $5,
                continents = $6,
                planets = $7,
                solar_systems = $8,
                galaxies = $9,
                universes = $10,
                prestige = $11,
                coal = $12,
                iron = $13,
                gold = $14,
                redstone = $15,
                lapis = $16,
                diamonds = $17,
                emeralds = $18,
                tech = $19,
                utility = $20,
                production = $21
            WHERE user_id = $1 AND prestige = $22;",
            row.user_id,
            row.miners,
            row.mines,
            row.land,
            row.countries,
            row.continents,
            row.planets,
            row.solar_systems,
            row.galaxies,
            row.universes,
            row.prestige,
            row.coal,
            row.iron,
            row.gold,
            row.redstone,
            row.lapis,
            row.diamonds,
            row.emeralds,
            row.tech,
            row.utility,
            row.production,
            expected_prestige,
        )
        .execute(&mut *tx)
        .await?;

        if mine.rows_affected() != 1 {
            tx.rollback().await?;
            return Ok(false);
        }

        sqlx::query!(
            "INSERT INTO gambling (user_id, coins, gems, stamina)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id) DO UPDATE SET
            coins = EXCLUDED.coins, gems = EXCLUDED.gems, stamina = EXCLUDED.stamina;",
            row.user_id,
            row.coins,
            row.gems,
            MAX_STAMINA,
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "DELETE FROM gambling_inventory
            WHERE user_id = $1;",
            row.user_id,
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(true)
    }
}

#[async_trait]
impl InventoryManager<Postgres> for PrestigeTable {
    async fn gambling_row(
        _pool: &PgPool,
        _id: UserId,
    ) -> sqlx::Result<Option<InventoryRow>> {
        Ok(None)
    }

    async fn inventory_items(
        pool: &PgPool,
        id: UserId,
    ) -> sqlx::Result<GamblingItems> {
        let items = sqlx::query_as!(
            GamblingItem,
            r#"SELECT item_id, quantity
            FROM gambling_inventory
            WHERE user_id = $1"#,
            as_i64(id.get())
        )
        .fetch_all(pool)
        .await?;

        Ok(GamblingItems(items))
    }

    async fn edit_item_quantity(
        _conn: &mut PgConnection,
        _id: impl Into<UserId> + Send,
        _item_id: &str,
        _amount: i64,
    ) -> sqlx::Result<i64> {
        Err(sqlx::Error::RowNotFound)
    }
}

pub struct Prestige;

#[async_trait]
impl ModuleCommand for Prestige {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("prestige")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_prestige()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        Commands::prestige::<Postgres, PrestigeTable>(
            cx.ctx,
            cx.interaction,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}

#[async_trait]
impl ModuleComponent for Prestige {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed("prestige"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        match cx.interaction.data.custom_id.as_str() {
            "prestige_confirm" => {
                Commands::confirm_prestige::<Postgres, PrestigeTable>(
                    cx.ctx,
                    cx.interaction,
                    &cx.app.db,
                    cx.app.zayden_id,
                )
                .await?;
            },
            "prestige_cancel" => {
                Commands::cancel_prestige(cx.ctx, cx.interaction).await?;
            },
            _ => {
                warn!(custom_id = %cx.interaction.data.custom_id, "unknown prestige component");
            },
        }

        Ok(())
    }
}
