use std::sync::Arc;

use futures::FutureExt;
use gambling::GamblingManager;
use serenity::all::{Context, Message};
use sqlx::{PgPool, Postgres};
use ticket::TicketStores;
use tracing::debug;
use zayden_app::state::AppState;

use crate::bindings::ai::Ai;
use crate::bindings::gambling::GamblingTable;
use crate::bindings::levels::LevelsTable;
use crate::bindings::ticket::message_commands::support;
use crate::handler::Handler;
use crate::{BotState, Result};

impl Handler {
    pub async fn message_create(
        ctx: &Context,
        msg: &Message,
        pool: &PgPool,
        app: Arc<AppState>,
    ) -> Result<()> {
        if msg.author.bot() {
            debug!(author_id = %msg.author.id, "message author is a bot; ignoring");
            return Ok(());
        }

        let stores = TicketStores {
            support: &app.settings.support,
            ticket: &app.settings.ticket,
        };

        if let Some(level) =
            levels::message_create::<Postgres, LevelsTable>(msg, pool).await?
        {
            let mut tx = pool.begin().await?;

            GamblingTable::add_coins(
                &mut tx,
                msg.author.id,
                i64::from(level) * 1000,
            )
            .await?;

            tx.commit().await?;
        }

        let (..) = tokio::try_join!(
            llamad2::GoodMorning::run::<BotState>(ctx, msg).map(Result::Ok),
            llamad2::BehindTheScenes::run(ctx, msg).map(Result::Ok),
            llamad2::CountingFail::run(ctx, msg).map(Result::Ok),
            Box::pin(support(&ctx.http, msg, stores, pool)),
            Box::pin(Ai::run(ctx, msg, &app)),
        )?;

        Ok(())
    }
}
