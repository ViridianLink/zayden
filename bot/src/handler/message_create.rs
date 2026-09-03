use std::sync::Arc;

use futures::FutureExt;
use gambling::{GamblingManager, level_up_reward};
use serenity::all::{Context, Message};
use sqlx::PgPool;
use tracing::debug;
use zayden_app::state::AppState;
use zayden_core::as_i64;

use crate::bindings::ai::Ai;
use crate::bindings::honeypot::record_hit;
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

        if let Some(guild_id) = msg.guild_id {
            let settings = app.settings.honeypot.get(as_i64(guild_id.get())).await?;

            if let Some(hit) = honeypot::message_create(ctx, msg, &settings).await? {
                record_hit(&app, &hit).await?;
                return Ok(());
            }
        }

        if let Some(level) = levels::message_create(msg, pool).await? {
            let reward = level_up_reward(i64::from(level));

            if reward > 0 {
                let mut tx = pool.begin().await?;

                GamblingManager::add_coins(&mut tx, msg.author.id, reward).await?;

                tx.commit().await?;
            }
        }

        let (..) = tokio::try_join!(
            llamad2::GoodMorning::run::<BotState>(ctx, msg).map(Result::Ok),
            llamad2::BehindTheScenes::run(ctx, msg).map(Result::Ok),
            llamad2::CountingFail::run(ctx, msg, pool).map(Result::Ok),
            Box::pin(support(&ctx.http, msg, &app)),
            Box::pin(Ai::run(ctx, msg, &app)),
        )?;

        Ok(())
    }
}
