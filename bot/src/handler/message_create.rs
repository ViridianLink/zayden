use std::sync::Arc;

use futures::{FutureExt, TryFutureExt};
use gambling::GamblingManager;
use serenity::all::{Context, Message};
use sqlx::{PgPool, Postgres};
use zayden_app::state::AppState;

use crate::bindings::ai::Ai;
use crate::bindings::gambling::GamblingTable;
use crate::bindings::levels::LevelsTable;
use crate::bindings::ticket::message_commands::support;
use crate::handler::Handler;
use crate::{BotError, BotState, Result};

impl Handler {
    pub async fn message_create(
        ctx: &Context,
        msg: &Message,
        pool: &PgPool,
        app: Arc<AppState>,
    ) -> Result<()> {
        if msg.author.bot() {
            debug!();
            return Ok(());
        }

        let (new_level, ..) = tokio::try_join!(
            levels::message_create::<Postgres, LevelsTable>(msg, pool)
                .map_err(BotError::from),
            llamad2::GoodMorning::run::<BotState>(ctx, msg).map(Result::Ok),
            llamad2::BehindTheScenes::run(ctx, msg).map(Result::Ok),
            llamad2::CountingFail::run(ctx, msg).map(Result::Ok),
            Box::pin(support(&ctx.http, msg, pool)),
            Box::pin(Ai::run(ctx, msg, &app)),
        )?;

        if let Some(level) = new_level {
            let mut tx = pool.begin().await?;

            GamblingTable::add_coins(
                &mut tx,
                msg.author.id,
                i64::from(level) * 1000,
            )
            .await?;

            tx.commit().await?;
        }

        Ok(())
    }
}
