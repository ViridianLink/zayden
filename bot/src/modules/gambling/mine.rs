use async_trait::async_trait;
use gambling::Commands;
use gambling::commands::mine::{MineManager, MineRow};
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::{PgPool, Postgres};
use zayden_core::SlashCommand;

use crate::{Error, Result};

pub struct MineTable;

#[async_trait]
impl MineManager<Postgres> for MineTable {
    async fn row(pool: &PgPool, id: impl Into<UserId> + Send) -> sqlx::Result<Option<MineRow>> {
        let id = id.into();

        sqlx::query_as!(
            MineRow,
            "SELECT miners, mines, land, countries, continents, planets, solar_systems, galaxies, universes, prestige FROM gambling_mine WHERE id = $1",
            id.get() as i64
        ).fetch_optional(pool).await
    }
}

pub struct Mine;

#[async_trait]
impl SlashCommand<Error, Postgres> for Mine {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::mine::<Postgres, MineTable>(&ctx.http, interaction, pool).await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(Commands::register_mine())
    }
}
