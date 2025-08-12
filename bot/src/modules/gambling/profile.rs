use async_trait::async_trait;
use gambling::commands::profile::{ProfileManager, ProfileRow};
use gambling::{Commands, GamblingItem};
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::types::Json;
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::{Error, Result};

pub struct ProfileTable;

#[async_trait]
impl ProfileManager<Postgres> for ProfileTable {
    async fn row(pool: &PgPool, id: impl Into<UserId> + Send) -> sqlx::Result<Option<ProfileRow>> {
        let id = id.into();

        sqlx::query_as!(
            ProfileRow,
            r#"SELECT
            g.coins,
            g.gems,

            COALESCE(l.xp, 0) AS xp,
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

            COALESCE(m.prestige, 0) as prestige
            
            FROM gambling g
            LEFT JOIN levels l ON g.id = l.id
            LEFT JOIN gambling_mine m on g.id = m.id
            WHERE g.id = $1;"#,
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }
}

pub struct Profile;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Profile {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::profile::<Postgres, ProfileTable>(&ctx.http, interaction, options, pool).await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(Commands::register_profile())
    }
}
