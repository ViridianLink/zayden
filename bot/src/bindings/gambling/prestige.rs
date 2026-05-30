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
use zayden_core::ctx::{ComponentCtx, InvocationCtx};
use zayden_core::error::HandlerError;
use zayden_core::module::{ModuleCommand, ModuleComponent};
use zayden_core::scope::IdMatch;

use super::stamina::MAX_STAMINA;
use crate::ZAYDEN_ID;

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
            id.get().cast_signed()
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
            id.get().cast_signed()
        )
        .fetch_optional(pool)
        .await
    }

    async fn lotto(pool: &PgPool, tickets: i64) -> sqlx::Result<PgQueryResult> {
        sqlx::query_file!(
            "./sql/gambling/PrestigeManager/lotto.sql",
            ZAYDEN_ID.get().cast_signed(),
            LOTTO_TICKET.id,
            tickets,
        )
        .execute(pool)
        .await
    }

    async fn save(pool: &PgPool, row: PrestigeRow) -> sqlx::Result<PgQueryResult> {
        let mut tx = pool.begin().await?;

        let mut result = sqlx::query!(
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

        let result2 = sqlx::query!(
            "DELETE FROM gambling_inventory
            WHERE user_id = $1;",
            row.user_id,
        )
        .execute(&mut *tx)
        .await?;

        let result3 = sqlx::query!(
            "INSERT INTO gambling_mine (user_id, miners, mines, land, countries, continents, planets, solar_systems, galaxies, universes, prestige, coal, iron, gold, redstone, lapis, diamonds, emeralds, tech, utility, production)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
            ON CONFLICT (user_id) DO UPDATE SET
                miners = EXCLUDED.miners,
                mines = EXCLUDED.mines,
                land = EXCLUDED.land,
                countries = EXCLUDED.countries,
                continents = EXCLUDED.continents,
                planets = EXCLUDED.planets,
                solar_systems = EXCLUDED.solar_systems,
                galaxies = EXCLUDED.galaxies,
                universes = EXCLUDED.universes,
                prestige = EXCLUDED.prestige,
                coal = EXCLUDED.coal,
                iron = EXCLUDED.iron,
                gold = EXCLUDED.gold,
                redstone = EXCLUDED.redstone,
                lapis = EXCLUDED.lapis,
                diamonds = EXCLUDED.diamonds,
                emeralds = EXCLUDED.emeralds,
                tech = EXCLUDED.tech,
                utility = EXCLUDED.utility,
                production = EXCLUDED.production;",
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
                row.production
            ).execute(&mut *tx)
        .await?;

        tx.commit().await?;

        result.extend([result2, result3]);

        Ok(result)
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
            id.get().cast_signed()
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
        .await
        .map_err(HandlerError::from_respond)
    }
}

#[async_trait]
impl ModuleComponent for Prestige {
    fn id_match(&self) -> IdMatch {
        IdMatch::Prefix(Cow::Borrowed("prestige"))
    }

    async fn run(&self, cx: &ComponentCtx<'_>) -> Result<(), HandlerError> {
        match cx.interaction.data.custom_id.as_str() {
            "prestige_confirm" => Commands::confirm_prestige::<
                Postgres,
                PrestigeTable,
            >(cx.ctx, cx.interaction, &cx.app.db)
            .await
            .map_err(HandlerError::from_respond),
            "prestige_cancel" => Commands::cancel_prestige(cx.ctx, cx.interaction)
                .await
                .map_err(HandlerError::from_respond),
            _ => {
                warn!(custom_id = %cx.interaction.data.custom_id, "unknown prestige component");
                Ok(())
            },
        }
    }
}
