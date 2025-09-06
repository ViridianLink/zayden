use async_trait::async_trait;
use gambling::commands::inventory::{InventoryManager, InventoryRow};
use gambling::{Commands, GamblingItem};
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::types::Json;
use sqlx::{PgConnection, PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::modules::gambling::EffectsTable;
use crate::{CtxData, Error, Result};

pub struct InventoryTable;

#[async_trait]
impl InventoryManager<Postgres> for InventoryTable {
    async fn row(
        pool: &PgPool,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<InventoryRow>> {
        let id = id.into();

        sqlx::query_as!(
            InventoryRow,
            r#"SELECT
            g.coins,
            g.gems,

            (
                SELECT jsonb_agg(
                    jsonb_build_object(
                        'quantity', inv.quantity,
                        'item_id', inv.item_id
                    )
                )
                FROM gambling_inventory inv
                WHERE inv.user_id = g.id
            ) as "inventory: Json<Vec<GamblingItem>>",

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

            FROM gambling g LEFT JOIN gambling_mine m ON g.id = m.id WHERE g.id = $1;"#,
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
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
            id.get() as i64,
            item_id,
            amount
        )
        .fetch_one(conn)
        .await
    }
}

pub struct Inventory;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Inventory {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::inventory::<CtxData, Postgres, EffectsTable, InventoryTable>(
            ctx,
            interaction,
            options,
            pool,
        )
        .await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(Commands::register_inventory())
    }
}
