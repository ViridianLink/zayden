use std::{collections::HashMap, sync::LazyLock};

use async_trait::async_trait;
use serenity::all::{ChannelId, Colour, CreateEmbed, CreateMessage, EmojiId, Mentionable, UserId};
use sqlx::{Database, Transaction};
use zayden_core::{CronJob, FormatNum};

use crate::{CARD_DECK, GEM, GamblingManager};

const CHANNEL_ID: ChannelId = ChannelId::new(1383573049563156502);

pub static CARD_TO_NUM: LazyLock<HashMap<EmojiId, u8>> = LazyLock::new(|| {
    CARD_DECK
        .iter()
        .copied()
        .zip((1u8..=13).cycle().take(52))
        .collect()
});

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

pub fn create_embed<'a>(seq: &str, payout: i64, winner: bool) -> CreateEmbed<'a> {
    let payout = payout.format();

    let desc = if winner {
        format!("{seq}\n\nCurrent Payout: {payout}\n\nGuess the next number!")
    } else {
        format!("{seq}\n\nFinal Payout: {payout}")
    };

    CreateEmbed::new()
        .title("Higher or Lower")
        .description(desc)
        .colour(Colour::TEAL)
}
