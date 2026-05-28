use std::borrow::Cow;

use async_trait::async_trait;
use gambling::commands::inventory::{InventoryManager, InventoryRow};
use gambling::commands::shop::SellRow;
use gambling::{Commands, GamblingItem, GamblingItems, ShopManager, ShopRow};
use serenity::all::{CreateCommand, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgConnection, PgPool, Postgres};
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;
use crate::bindings::gambling::GoalsTable;

pub struct ShopTable;

#[async_trait]
impl ShopManager<Postgres> for ShopTable {
    async fn buy_row(pool: &PgPool, id: impl Into<UserId> + Send) -> sqlx::Result<Option<ShopRow>> {
        let id = id.into();

        sqlx::query_as!(ShopRow,
            r#"SELECT
            g.user_id,
            g.coins,
            g.gems,
            
            COALESCE(l.level, 0) AS level,

            COALESCE(m.miners, 0) AS "miners!",
            COALESCE(m.mines, 0) AS "mines!",
            COALESCE(m.land, 0) AS "land!",
            COALESCE(m.countries, 0) AS "countries!",
            COALESCE(m.continents, 0) AS "continents!",
            COALESCE(m.planets, 0) AS "planets!",
            COALESCE(m.solar_systems, 0) AS "solar_systems!",
            COALESCE(m.galaxies, 0) AS "galaxies!",
            COALESCE(m.universes, 0) AS "universes!",
            COALESCE(m.prestige, 0) AS "prestige!",
            COALESCE(m.tech, 0) AS "tech!",
            COALESCE(m.utility, 0) AS "utility!",
            COALESCE(m.production, 0) AS "production!"

            FROM gambling g LEFT JOIN levels l ON g.user_id = l.user_id LEFT JOIN gambling_mine m ON g.user_id = m.user_id WHERE g.user_id = $1;"#,
            id.get() as i64
        ).fetch_optional(pool).await
    }

    async fn buy_save(pool: &PgPool, row: ShopRow) -> sqlx::Result<PgQueryResult> {
        let mut tx = pool.begin().await?;

        let mut result = sqlx::query!(
            "INSERT INTO gambling (user_id, coins, gems)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id) DO UPDATE SET
            coins = EXCLUDED.coins, gems = EXCLUDED.gems;",
            row.user_id,
            row.coins,
            row.gems,
        )
        .execute(&mut *tx)
        .await?;

        let result3 = sqlx::query!(
            "INSERT INTO gambling_mine (user_id, miners, mines, land, countries, continents, planets, solar_systems, galaxies, universes, tech, utility, production)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (user_id) DO UPDATE
            SET
            miners = EXCLUDED.miners,
            mines = EXCLUDED.mines,
            land = EXCLUDED.land,
            countries = EXCLUDED.countries,
            continents = EXCLUDED.continents,
            planets = EXCLUDED.planets,
            solar_systems = EXCLUDED.solar_systems,
            galaxies = EXCLUDED.galaxies,
            universes = EXCLUDED.universes,
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
            row.tech,
            row.utility,
            row.production,
        ).execute(&mut *tx).await?;

        result.extend([result3]);

        tx.commit().await.unwrap();

        Ok(result)
    }

    async fn save_inventory(
        pool: &PgPool,
        user_id: UserId,
        rows: GamblingItems,
    ) -> sqlx::Result<PgQueryResult> {
        let mut item_ids = Vec::with_capacity(rows.0.len());
        let mut quantities = Vec::with_capacity(rows.0.len());

        for item in rows.0 {
            item_ids.push(item.item_id);
            quantities.push(item.quantity);
        }

        sqlx::query!(
            "INSERT INTO gambling_inventory (user_id, item_id, quantity)
            SELECT $1, * FROM UNNEST($2::text[], $3::bigint[])
            ON CONFLICT (user_id, item_id) DO UPDATE
            SET quantity = EXCLUDED.quantity",
            user_id.get() as i64,
            &item_ids,
            &quantities
        )
        .execute(pool)
        .await
    }

    async fn sell_row(
        pool: &PgPool,
        id: impl Into<UserId> + Send,
        item_id: &str,
    ) -> sqlx::Result<Option<SellRow>> {
        let id = id.into();

        sqlx::query_as!(
            SellRow,
            r#"
            SELECT
                g.user_id,
                g.coins,

                i.id AS "item_row_id?",
                i.quantity AS "item_quantity?"
            FROM
                gambling g
            LEFT JOIN
                gambling_inventory i ON g.user_id = i.user_id AND i.item_id = $2
            WHERE
                g.user_id = $1
            "#,
            id.get() as i64,
            item_id
        )
        .fetch_optional(pool)
        .await
    }

    async fn sell_save(pool: &PgPool, row: SellRow) -> sqlx::Result<PgQueryResult> {
        let mut tx = pool.begin().await?;

        let mut result = sqlx::query!(
            "INSERT INTO gambling (user_id, coins)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE SET
            coins = EXCLUDED.coins;",
            row.user_id,
            row.coins,
        )
        .execute(&mut *tx)
        .await?;

        let result2 = if row.item_quantity == Some(0) {
            sqlx::query!(
                "DELETE FROM gambling_inventory WHERE id = $1",
                row.item_row_id
            )
            .execute(&mut *tx)
            .await?
        } else {
            sqlx::query!(
                "UPDATE gambling_inventory SET quantity = $1 WHERE id = $2",
                row.item_quantity,
                row.item_row_id
            )
            .execute(&mut *tx)
            .await?
        };

        result.extend([result2]);

        tx.commit().await?;

        Ok(result)
    }
}

#[async_trait]
impl InventoryManager<Postgres> for ShopTable {
    async fn gambling_row(_pool: &PgPool, _id: UserId) -> sqlx::Result<Option<InventoryRow>> {
        unimplemented!()
    }
    async fn inventory_items(pool: &PgPool, id: UserId) -> sqlx::Result<GamblingItems> {
        let items = sqlx::query_as!(
            GamblingItem,
            r#"SELECT item_id, quantity
            FROM gambling_inventory
            WHERE user_id = $1"#,
            id.get() as i64
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
        unimplemented!()
    }
}

pub struct Shop;

#[async_trait]
impl ModuleCommand for Shop {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("shop")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_shop()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        Commands::shop::<BotState, Postgres, GoalsTable, ShopTable>(
            cx.ctx,
            cx.interaction,
            options,
            &cx.app.db,
        )
        .await
        .map_err(HandlerError::from_respond)
    }
}
