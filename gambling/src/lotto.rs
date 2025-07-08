use async_trait::async_trait;
use rand::distr::weighted::WeightedIndex;
use rand::rng;
use rand_distr::Distribution;
use serenity::all::{ChannelId, CreateEmbed, CreateMessage, Mentionable, UserId};
use sqlx::any::AnyQueryResult;
use sqlx::{Database, FromRow};
use zayden_core::{CronJob, FormatNum};

use crate::shop::LOTTO_TICKET;
use crate::{COIN, Coins, GamblingManager};

const CHANNEL_ID: ChannelId = ChannelId::new(1383573049563156502);

#[async_trait]
pub trait LottoManager<Db: Database> {
    async fn row(
        conn: &mut Db::Connection,
        id: impl Into<UserId> + Send,
    ) -> sqlx::Result<Option<LottoRow>>;

    async fn rows(conn: &mut Db::Connection) -> sqlx::Result<Vec<LottoRow>>;

    async fn total_tickets(conn: &mut Db::Connection) -> sqlx::Result<i64>;

    async fn delete_tickets(conn: &mut Db::Connection) -> sqlx::Result<AnyQueryResult>;
}

#[derive(FromRow)]
pub struct LottoRow {
    pub id: i64,
    pub coins: i64,
    pub quantity: Option<i64>,
}

impl LottoRow {
    pub fn new(id: impl Into<UserId> + Send) -> Self {
        let id = id.into();

        Self {
            id: id.get() as i64,
            coins: 0,
            quantity: Some(0),
        }
    }

    fn user_id(&self) -> UserId {
        UserId::new(self.id as u64)
    }

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
pub fn jackpot(tickets: i64) -> i64 {
    (tickets * LOTTO_TICKET.coin_cost().unwrap()).max(1_000_000)
}

pub struct Lotto;

impl Lotto {
    pub fn cron_job<
        Db: Database,
        GamblingHandler: GamblingManager<Db>,
        LottoHandler: LottoManager<Db>,
    >() -> CronJob<Db> {
        CronJob::new("lotto", "0 0 17 * * Fri *").set_action(|ctx, pool| async move {
            let mut tx: sqlx::Transaction<'static, Db> = pool.begin().await.unwrap();

            let mut rows = LottoHandler::rows(&mut *tx).await.unwrap();

            let prize_share = [0.5, 0.3, 0.2];

            let expected_winners = prize_share.len();

            if rows.len() < expected_winners {
                return;
            }

            let total_tickets: i64 = rows.iter().map(|row| row.quantity()).sum();

            let mut dist = WeightedIndex::new(rows.iter().map(|row| row.quantity())).unwrap();

            let jackpot = jackpot(total_tickets);

            let winners = prize_share
                .into_iter()
                .map(|share| {
                    let index = dist.sample(&mut rng());
                    let winner = rows.remove(index);
                    dist = WeightedIndex::new(rows.iter().map(|row| row.quantity())).unwrap();
                    (winner.user_id(), (jackpot as f64 * share) as i64)
                })
                .collect::<Vec<_>>();

            LottoHandler::delete_tickets(&mut *tx).await.unwrap();

            let mut lines = Vec::with_capacity(expected_winners);

            for (winner, payout) in winners {
                GamblingHandler::add_coins(&mut *tx, winner, payout)
                    .await
                    .unwrap();

                let line = format!(
                    "{} ({}) has won {} <:coin:{COIN}> from the lottery!",
                    winner.mention(),
                    winner.to_user(&ctx).await.unwrap().display_name(),
                    payout.format()
                );

                lines.push(line);
            }

            tx.commit().await.unwrap();

            let embed = CreateEmbed::new()
                .title(format!(
                    "<:coin:{COIN}> <:coin:{COIN}> Lottery!! <:coin:{COIN}> <:coin:{COIN}>"
                ))
                .field(
                    "Tickets Bought",
                    format!("{} {}", total_tickets.format(), LOTTO_TICKET.emoji()),
                    false,
                )
                .field(
                    "Jackpot Value",
                    format!("{} <:coin:{COIN}>", jackpot.format()),
                    false,
                );

            CHANNEL_ID
                .send_message(
                    &ctx,
                    CreateMessage::new().content(lines.join("\n")).embed(embed),
                )
                .await
                .unwrap()
                .crosspost(&ctx)
                .await
                .unwrap();
        })
    }
}
