use async_trait::async_trait;
use gambling::commands::inventory::{InventoryManager, InventoryRow};
use gambling::commands::profile::{ProfileManager, ProfileRow};
use gambling::{Commands, GamblingItem, GamblingItems};
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption, UserId};
use sqlx::{PgConnection, PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::{CtxData, Error, Result};

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

            COALESCE(m.prestige, 0) as prestige
            
            FROM gambling g
            LEFT JOIN levels l ON g.user_id = l.user_id
            LEFT JOIN gambling_mine m on g.user_id = m.user_id
            WHERE g.user_id = $1;"#,
            id.get() as i64
        )
        .fetch_optional(pool)
        .await
    }
}

#[async_trait]
impl InventoryManager<Postgres> for ProfileTable {
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

pub struct Profile;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Profile {
    async fn run(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        Commands::profile::<CtxData, Postgres, ProfileTable>(ctx, interaction, options, pool)
            .await?;

        Ok(())
    }

    fn command(&self) -> CreateCommand<'_> {
        Commands::register_profile()
    }
}
