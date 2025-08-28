use async_trait::async_trait;
use serenity::all::{CommandInteraction, Context, CreateCommand, ResolvedOption};
use sqlx::{PgPool, Postgres};
use tokio::sync::OnceCell;
use zayden_core::ApplicationCommand;

use crate::{CtxData, Error, Result};

static ZAYDEN_TOKEN: OnceCell<String> = OnceCell::const_new();

async fn zayden_token(pool: &PgPool) -> String {
    sqlx::query_scalar!("SELECT token FROM bot_tokens WHERE name = 'zayden'")
        .fetch_one(pool)
        .await
        .unwrap()
}

pub struct Loadout;

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Loadout {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        let zayden_token = ZAYDEN_TOKEN.get_or_init(|| zayden_token(pool)).await;

        destiny2::loadouts::Loadout::run::<CtxData>(ctx, interaction, options, zayden_token).await;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand<'_>> {
        Ok(destiny2::loadouts::Loadout::register())
    }
}
