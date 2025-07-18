use async_trait::async_trait;
use serenity::all::{ChannelId, CreateMessage, Mentionable, UserId};
use sqlx::{Database, Transaction};
use zayden_core::{CronJob, FormatNum};

use crate::{GEM, GamblingManager};

const CHANNEL_ID: ChannelId = ChannelId::new(1383573049563156502);

#[async_trait]
pub trait HigherLowerManager<Db: Database> {
    async fn winners(conn: &mut Db::Connection) -> sqlx::Result<Vec<UserId>>;
    async fn reset(conn: &mut Db::Connection) -> sqlx::Result<Db::QueryResult>;
}

pub struct HigherLower;

impl HigherLower {
    pub fn cron_job<
        Db: Database,
        GamblingHandler: GamblingManager<Db>,
        HigherLowerHandler: HigherLowerManager<Db>,
    >() -> CronJob<Db> {
        CronJob::new("lotto", "0 0 17 * * Fri *").set_action(|ctx, pool| async move {
            let mut tx: Transaction<'_, Db> = pool.begin().await.unwrap();

            let winners = HigherLowerHandler::winners(&mut *tx).await.unwrap();
            HigherLowerHandler::reset(&mut *tx).await.unwrap();

            let mut lines = Vec::with_capacity(3);
            for (winner, payout) in winners.into_iter().zip([3, 2, 1]) {
                GamblingHandler::add_gems(&mut tx, winner, payout)
                    .await
                    .unwrap();

                let user = winner.to_user(&ctx.http).await.unwrap();

                let line = format!(
                    "{} ({}) has won {} {GEM} from the weekly higher or lower leaderboard!",
                    user.mention(),
                    user.display_name(),
                    payout.format()
                );

                lines.push(line);
            }

            tx.commit().await.unwrap();

            CHANNEL_ID
                .widen()
                .send_message(&ctx.http, CreateMessage::new().content(lines.join("\n")))
                .await
                .unwrap()
                .crosspost(&ctx.http)
                .await
                .unwrap();
        })
    }
}
