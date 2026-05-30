use async_trait::async_trait;
use rand::distr::weighted::WeightedIndex;
use rand::rng;
use rand_distr::Distribution;
use serenity::all::{ChannelId, CreateEmbed, CreateMessage, Mentionable, UserId};
use sqlx::{Database, FromRow};
use tokio::sync::RwLock;
use tracing::error;
use zayden_core::{CronJob, EmojiCacheData, FormatNum};

use crate::shop::LOTTO_TICKET;
use crate::{Coins, GamblingManager, bot_id};

const CHANNEL_ID: ChannelId = ChannelId::new(1_383_573_049_563_156_502);

#[async_trait]
pub trait LottoManager<Db: Database> {
    async fn row(
        conn: &mut Db::Connection,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<LottoRow>>;

    async fn rows(conn: &mut Db::Connection) -> sqlx::Result<Vec<LottoRow>>;

    async fn total_tickets(conn: &mut Db::Connection) -> sqlx::Result<i64>;

    async fn delete_tickets(
        conn: &mut Db::Connection,
    ) -> sqlx::Result<Db::QueryResult>;
}

#[derive(FromRow)]
pub struct LottoRow {
    pub user_id: i64,
    pub coins: i64,
    pub quantity: Option<i64>,
}

impl LottoRow {
    pub fn new(id: impl Into<UserId> + Send) -> Self {
        let id: UserId = id.into();

        Self { user_id: id.get().cast_signed(), coins: 0, quantity: Some(0) }
    }

    const fn user_id(&self) -> UserId {
        UserId::new(self.user_id.cast_unsigned())
    }

    #[must_use]
    pub fn quantity(&self) -> i64 {
        self.quantity.unwrap_or(0)
    }
}

impl Coins for LottoRow {
    fn coins(&self) -> i64 {
        self.coins
    }

    fn coins_mut(&mut self) -> &mut i64 {
        &mut self.coins
    }
}

#[inline]
#[must_use]
pub fn jackpot(tickets: i64) -> i64 {
    tickets
        .saturating_mul(
            LOTTO_TICKET.coin_cost().expect("LOTTO_TICKET has a coin cost"),
        )
        .max(1_000_000)
}

pub struct Lotto;

impl Lotto {
    pub fn cron_job<
        Data: EmojiCacheData,
        Db: Database,
        GamblingHandler: GamblingManager<Db>,
        LottoHandler: LottoManager<Db>,
    >() -> Result<CronJob<Db>, jiff_cron::error::Error> {
        Ok(CronJob::new("lotto", "0 0 17 * * Fri *")?.set_action(|ctx, pool| async move {
            let bot_id = bot_id(&ctx.http).await;

            if let Err(e) = (async {
                let mut tx: sqlx::Transaction<'static, Db> = pool.begin().await?;

                let mut rows = LottoHandler::rows(&mut *tx).await?;

                let total_tickets: i64 = rows.iter().map(LottoRow::quantity).sum();

                rows.retain(|row| row.user_id.cast_unsigned() != bot_id.get());

                let prize_share = [0.5, 0.3, 0.2];
                let expected_winners = prize_share.len();

                if rows.len() < expected_winners {
                    return Ok(());
                }

                let mut dist = WeightedIndex::new(rows.iter().map(LottoRow::quantity))
                    .expect("weighted index valid");

                let jackpot = jackpot(total_tickets);

                let winners = prize_share
                    .into_iter()
                    .map(|share| {
                        let index = dist.sample(&mut rng());
                        let winner = rows.remove(index);
                        dist = WeightedIndex::new(rows.iter().map(LottoRow::quantity))
                            .expect("weighted index valid");
                        #[expect(
                            clippy::cast_possible_truncation,
                            clippy::cast_precision_loss,
                            reason = "lottery payout: precision/truncation acceptable"
                        )]
                        let payout = (jackpot as f64 * share) as i64;
                        (winner.user_id(), payout)
                    })
                    .collect::<Vec<_>>();

                LottoHandler::delete_tickets(&mut *tx).await?;

                let emojis = {
                    let data = ctx.data::<RwLock<Data>>();
                    let data = data.read().await;
                    data.emojis()
                };

                let coin = emojis.emoji("heads").expect("emoji 'heads' in cache");

                let mut lines = Vec::with_capacity(expected_winners);

                for (winner, payout) in winners {
                    if let Err(e) = GamblingHandler::add_coins(&mut *tx, winner, payout).await {
                        error!("Lotto job crashed: {e}");
                        return Ok(());
                    }

                    let line = format!(
                        "{} ({}) has won {} <:coin:{coin}> from the lottery!",
                        winner.mention(),
                        winner.to_user(&ctx).await.expect("async call").display_name(),
                        payout.format()
                    );

                    lines.push(line);
                }

                tx.commit().await?;

                let embed = CreateEmbed::new()
                    .title(format!(
                        "<:coin:{coin}> <:coin:{coin}> Lottery!! <:coin:{coin}> <:coin:{coin}>"
                    ))
                    .field(
                        "Tickets Bought",
                        format!("{} {}", total_tickets.format(), LOTTO_TICKET.emoji(&emojis)),
                        false,
                    )
                    .field(
                        "Jackpot Value",
                        format!("{} <:coin:{coin}>", jackpot.format()),
                        false,
                    );

                CHANNEL_ID
                    .widen()
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().content(lines.join("\n")).embed(embed),
                    )
                    .await?;

                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            })
            .await
            {
                error!("lotto cron job failed: {e}");
            }
        }))
    }
}
