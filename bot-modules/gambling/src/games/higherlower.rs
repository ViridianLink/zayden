use futures::TryStreamExt;
use jiff_cron;
use serenity::all::{
    ChannelId,
    Colour,
    CreateEmbed,
    CreateMessage,
    Mentionable,
    UserId,
};
use sqlx::postgres::PgQueryResult;
use sqlx::{PgConnection, Postgres, Transaction};
use zayden_core::{CronJob, FormatNum, as_u64};

use crate::{GEM, GamblingManager};

const CHANNEL_ID: ChannelId = ChannelId::new(1_383_573_049_563_156_502);

pub struct HigherLowerManager;

impl HigherLowerManager {
    pub async fn winners(conn: &mut PgConnection) -> sqlx::Result<Vec<UserId>> {
        sqlx::query_file_scalar!("sql/HigherLowerManager/winners.sql")
            .fetch(conn)
            .map_ok(|id| UserId::new(as_u64(id)))
            .try_collect()
            .await
    }

    pub async fn reset(conn: &mut PgConnection) -> sqlx::Result<PgQueryResult> {
        sqlx::query_file_scalar!("sql/HigherLowerManager/reset.sql")
            .execute(conn)
            .await
    }
}

pub struct HigherLower;

impl HigherLower {
    pub fn cron_job() -> Result<CronJob, jiff_cron::error::Error> {
        Ok(CronJob::new("lotto", "0 0 17 * * Fri *")?.set_action(|ctx, pool| async move {
            if let Err(e) = (async {
                let mut tx: Transaction<'_, Postgres> = pool.begin().await?;

                let winners = HigherLowerManager::winners(&mut tx).await?;
                HigherLowerManager::reset(&mut tx).await?;

                let mut lines = Vec::with_capacity(3);
                for (winner, payout) in winners.into_iter().zip([3, 2, 1]) {
                    GamblingManager::add_gems(&mut tx, winner, payout).await?;

                    let user = winner.to_user(&ctx.http).await?;

                    let line = format!(
                        "{} ({}) has won {} {GEM} from the weekly higher or lower leaderboard!",
                        user.mention(),
                        user.display_name(),
                        payout.format()
                    );

                    lines.push(line);
                }

                tx.commit().await?;

                CHANNEL_ID
                    .widen()
                    .send_message(&ctx.http, CreateMessage::new().content(lines.join("\n")))
                    .await?;

                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            })
            .await
            {
                tracing::error!(error = ?e, "higher_lower cron job failed");
            }
        }))
    }
}

pub fn create_embed<'a>(seq: &str, payout: i64, winner: bool) -> CreateEmbed<'a> {
    let payout = payout.format();

    let desc = if winner {
        format!("# {seq}\n\nCurrent Payout: {payout}\n\nGuess the next number!")
    } else {
        format!("{seq}\n\nFinal Payout: {payout}")
    };

    CreateEmbed::new()
        .title("Higher or Lower")
        .description(desc)
        .colour(Colour::TEAL)
}
