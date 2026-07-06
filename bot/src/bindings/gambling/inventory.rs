use std::borrow::Cow;

use async_trait::async_trait;
use gambling::commands::inventory::{InventoryManager, InventoryRow};
use gambling::{Commands, EffectsTable, GamblingItem, GamblingItems};
use serenity::all::{CreateCommand, UserId};
use sqlx::{PgConnection, PgPool, Postgres};
use zayden_core::as_i64;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;

pub struct InventoryTable;

#[async_trait]
impl InventoryManager<Postgres> for InventoryTable {
    async fn gambling_row(
        pool: &PgPool,
        id: UserId,
    ) -> sqlx::Result<Option<InventoryRow>> {
        sqlx::query_as!(
            InventoryRow,
            r#"SELECT
            g.coins,
            g.gems,

            COALESCE(m.tech, 0) AS "tech!",
            COALESCE(m.utility, 0) AS "utility!",
            COALESCE(m.production, 0) AS "production!",
            COALESCE(m.coal, 0) AS "coal!",
            COALESCE(m.iron, 0) AS "iron!",
            COALESCE(m.gold, 0) AS "gold!",
            COALESCE(m.redstone, 0) AS "redstone!",
            COALESCE(m.lapis, 0) AS "lapis!",
            COALESCE(m.diamonds, 0) AS "diamonds!",
            COALESCE(m.emeralds, 0) AS "emeralds!"

            FROM gambling g LEFT JOIN gambling_mine m ON g.user_id = m.user_id WHERE g.user_id = $1"#,
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
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
        conn: &mut PgConnection,
        id: impl Into<UserId> + Send,
        item_id: &str,
        amount: i64,
    ) -> sqlx::Result<i64> {
        let id = id.into();

        sqlx::query_scalar!(
            r#"
        WITH updated_row AS (
            UPDATE gambling_inventory
            SET quantity = quantity - $3
            WHERE user_id = $1
              AND item_id = $2
              AND $3 <= gambling_inventory.quantity
            RETURNING quantity
        ),
        deleted_row AS (
            DELETE FROM gambling_inventory
            WHERE user_id = $1 AND item_id = $2
            AND EXISTS (SELECT 1 FROM updated_row ur WHERE ur.quantity <= 0)
            RETURNING item_id
        )
        SELECT
            ur.quantity
        FROM
            updated_row ur
        "#,
            as_i64(id.get()),
            item_id,
            amount
        )
        .fetch_one(conn)
        .await
    }
}

pub struct Inventory;

#[async_trait]
impl ModuleCommand for Inventory {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("inventory")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_inventory()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        Commands::inventory::<BotState, Postgres, EffectsTable, InventoryTable>(
            cx.ctx,
            cx.interaction,
            options,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}
