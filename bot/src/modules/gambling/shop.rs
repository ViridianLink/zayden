use async_trait::async_trait;
use gambling::commands::shop::{BuyRow, SellRow, ShopManager};
use gambling::{Commands, GamblingItem};
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::types::Json;
use sqlx::{PgPool, Postgres};
use zayden_core::SlashCommand;

use crate::modules::gambling::GoalsTable;
use crate::{Error, Result};

pub struct ShopTable;

#[async_trait]
impl ShopManager<Postgres> for ShopTable {
    async fn buy_row(pool: &PgPool, id: impl Into<UserId> + Send) -> sqlx::Result<Option<BuyRow>> {
        let id = id.into();

        sqlx::query_as!(BuyRow,
            r#"SELECT
            g.id,
            g.coins,
            g.gems,
            
            COALESCE(l.level, 0) AS level,

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

            FROM gambling g LEFT JOIN levels l ON g.id = l.id LEFT JOIN gambling_mine m ON g.id = m.id WHERE g.id = $1;"#,
            id.get() as i64
        ).fetch_optional(pool).await
    }

    async fn buy_save(pool: &PgPool, row: BuyRow) -> sqlx::Result<PgQueryResult> {
        let mut tx = pool.begin().await?;

        let mut result = sqlx::query!(
            "INSERT INTO gambling (id, coins, gems)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO UPDATE SET
            coins = EXCLUDED.coins, gems = EXCLUDED.gems;",
            row.id,
            row.coins,
            row.gems,
        )
        .execute(&mut *tx)
        .await?;

        for item in row.inventory.unwrap_or_default().0 {
            let result2 = sqlx::query!(
                "INSERT INTO gambling_inventory (user_id, item_id, quantity)
                VALUES ($1, $2, $3)
                ON CONFLICT (user_id, item_id) DO UPDATE
                SET quantity = EXCLUDED.quantity",
                row.id,
                item.item_id,
                item.quantity
            )
            .execute(&mut *tx)
            .await?;

            result.extend([result2]);
        }

        let result3 = sqlx::query!(
            "INSERT INTO gambling_mine (id, miners, mines, land, countries, continents, planets, solar_systems, galaxies, universes, tech, utility, production)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (id) DO UPDATE
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
            row.id,
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
                g.id,
                g.coins,

                i.id AS "item_row_id?",
                i.quantity AS "item_quantity?"
            FROM
                gambling g
            LEFT JOIN
                gambling_inventory i ON g.id = i.user_id AND i.item_id = $2
            WHERE
                g.id = $1
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
            "INSERT INTO gambling (id, coins)
            VALUES ($1, $2)
            ON CONFLICT (id) DO UPDATE SET
            coins = EXCLUDED.coins;",
            row.id,
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

pub struct Shop;

#[async_trait]
impl SlashCommand<Error, Postgres> for Shop {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::shop::<Postgres, GoalsTable, ShopTable>(ctx, interaction, options, pool).await?;
        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand> {
        Ok(Commands::register_shop())
    }
}
