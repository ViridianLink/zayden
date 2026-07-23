use std::borrow::Cow;

use async_trait::async_trait;
use gambling::commands::inventory::{InventoryManager, InventoryRow};
use gambling::commands::profile::{ProfileManager, ProfileRow};
use gambling::{Commands, GamblingItem, GamblingItems};
use serenity::all::{CreateCommand, UserId};
use sqlx::{PgConnection, PgPool, Postgres};
use zayden_core::as_i64;
use zayden_core::ctx::InvocationCtx;
use zayden_core::error::HandlerError;
use zayden_core::module::ModuleCommand;

use crate::BotState;

pub struct ProfileTable;

#[async_trait]
impl ProfileManager<Postgres> for ProfileTable {
    async fn row(pool: &PgPool, id: UserId) -> sqlx::Result<Option<ProfileRow>> {
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
            as_i64(id.get())
        )
        .fetch_optional(pool)
        .await
    }
}

#[async_trait]
impl InventoryManager<Postgres> for ProfileTable {
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
        _id: UserId,
        _item_id: &str,
        _amount: i64,
    ) -> sqlx::Result<i64> {
        Err(sqlx::Error::RowNotFound)
    }
}

pub struct Profile;

#[async_trait]
impl ModuleCommand for Profile {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("profile")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Commands::register_profile()
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        let options = cx.interaction.data.options();
        Commands::profile::<BotState, Postgres, ProfileTable>(
            cx.ctx,
            cx.interaction,
            options,
            &cx.app.db,
        )
        .await?;
        Ok(())
    }
}
