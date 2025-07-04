use async_trait::async_trait;
use gambling::commands::prestige::{PrestigeManager, PrestigeRow};
use gambling::shop::LOTTO_TICKET;
use gambling::{Commands, GamblingItem};
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::postgres::PgQueryResult;
use sqlx::types::Json;
use sqlx::{PgPool, Postgres};
use zayden_core::SlashCommand;

use crate::{Error, Result};

use super::stamina::MAX_STAMINA;

pub struct PrestigeTable;

#[async_trait]
impl PrestigeManager<Postgres> for PrestigeTable {
    async fn miners(pool: &PgPool, id: impl Into<UserId> + Send) -> sqlx::Result<Option<i64>> {
        let id = id.into();

        sqlx::query_scalar!(
            "SELECT miners FROM gambling_mine WHERE id = $1;",
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }

    async fn row(pool: &PgPool, id: impl Into<UserId> + Send) -> sqlx::Result<Option<PrestigeRow>> {
        let id = id.into();

        sqlx::query_as!(
            PrestigeRow,
            r#"SELECT
                g.id,
                g.coins,
                g.gems,
                g.stamina,
                
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
                
                m.miners,
                m.mines,
                m.land,
                m.countries,
                m.continents,
                m.planets,
                m.solar_systems,
                m.galaxies,
                m.universes,
                m.prestige,
                m.coal,
                m.iron,
                m.gold,
                m.redstone,
                m.lapis,
                m.diamonds,
                m.emeralds,
                m.tech,
                m.utility,
                m.production

                FROM gambling g
                LEFT JOIN gambling_inventory i on g.id = i.id
                LEFT JOIN gambling_mine m on g.id = m.id
                WHERE g.id = $1;"#,
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }

    async fn lotto(pool: &PgPool, tickets: i64) -> sqlx::Result<PgQueryResult> {
        const BOT_ID: i64 = 787490197943091211;

        sqlx::query_file!(
            "./sql/gambling/PrestigeManager/lotto.sql",
            BOT_ID,
            LOTTO_TICKET.id,
            tickets,
        )
        .execute(pool)
        .await
    }

    async fn save(pool: &PgPool, row: PrestigeRow) -> sqlx::Result<PgQueryResult> {
        let mut tx = pool.begin().await?;

        let mut result = sqlx::query!(
            "INSERT INTO gambling (id, coins, gems, stamina)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (id) DO UPDATE SET
            coins = EXCLUDED.coins, gems = EXCLUDED.gems, stamina = EXCLUDED.stamina;",
            row.id,
            row.coins,
            row.gems,
            MAX_STAMINA,
        )
        .execute(&mut *tx)
        .await?;

        let result2 = sqlx::query!(
            "DELETE FROM gambling_inventory
            WHERE user_id = $1;",
            row.id,
        )
        .execute(&mut *tx)
        .await?;

        let result3 = sqlx::query!(
            "INSERT INTO gambling_inventory (user_id, item_id, quantity)
            SELECT
                $1 AS user_id,
                (elem->>'item_id')::TEXT AS item_id,
                (elem->>'quantity')::INTEGER AS quantity
            FROM
                jsonb_array_elements($2::JSONB) AS elem;",
            row.id,
            serde_json::to_value(row.inventory.unwrap_or_default().0).unwrap()
        )
        .execute(&mut *tx)
        .await?;

        let result4 = sqlx::query!(
            "INSERT INTO gambling_mine (id, miners, mines, land, countries, continents, planets, solar_systems, galaxies, universes, prestige, coal, iron, gold, redstone, lapis, diamonds, emeralds, tech, utility, production)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21)
            ON CONFLICT (id) DO UPDATE SET
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

        result.extend([result2, result3, result4]);

        Ok(result)
    }
}

pub struct Prestige;

#[async_trait]
impl SlashCommand<Error, Postgres> for Prestige {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::prestige::<Postgres, PrestigeTable>(ctx, interaction, pool).await?;
        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand> {
        Ok(Commands::register_prestige())
    }
}
