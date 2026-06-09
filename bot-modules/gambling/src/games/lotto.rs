use async_trait::async_trait;
use rand::distr::weighted::WeightedIndex;
use rand::rng;
use rand_distr::Distribution;
use serenity::all::{ChannelId, CreateEmbed, CreateMessage, Mentionable, UserId};
use sqlx::{Database, FromRow};
use tokio::sync::RwLock;
use tracing::{debug, error};
use zayden_core::{CronJob, EmojiCacheData, FormatNum, as_i64, as_u64};

use crate::shop::LOTTO_TICKET;
use crate::{Coins, GamblingError, GamblingManager, bot_id};

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

        Self { user_id: as_i64(id.get()), coins: 0, quantity: Some(0) }
    }

    const fn user_id(&self) -> UserId {
        UserId::new(as_u64(self.user_id))
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
    tickets.saturating_mul(LOTTO_TICKET.coin_cost().unwrap_or(0)).max(1_000_000)
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
            if let Err(e) = (async {
                let bot_id = bot_id(&ctx.http)
                    .await
                    .map_err(|e| GamblingError::Internal(format!("bot_id fetch failed: {e}")))?;

                let mut tx: sqlx::Transaction<'static, Db> = pool.begin().await?;

                let mut rows = LottoHandler::rows(&mut *tx).await?;

                let total_tickets: i64 = rows.iter().map(LottoRow::quantity).sum();

                rows.retain(|row| as_u64(row.user_id) != bot_id.get());

                let prize_share = [0.5, 0.3, 0.2];
                let expected_winners = prize_share.len();

                if rows.len() < expected_winners {
                    debug!("fewer eligible participants than prize tiers - skipping");
                    return Ok(());
                }

                let mut dist =
                    WeightedIndex::new(rows.iter().map(LottoRow::quantity)).map_err(|e| {
                        GamblingError::Internal(format!("WeightedIndex creation failed: {e}"))
                    })?;

                let jackpot = jackpot(total_tickets);

                let mut winners = Vec::with_capacity(expected_winners);
                for share in prize_share {
                    let index = dist.sample(&mut rng());
                    let winner = rows.remove(index);
                    dist =
                        WeightedIndex::new(rows.iter().map(LottoRow::quantity)).map_err(|e| {
                            GamblingError::Internal(format!("WeightedIndex update failed: {e}"))
                        })?;
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_precision_loss,
                        reason = "lottery payout: precision/truncation acceptable"
                    )]
                    let payout = (jackpot as f64 * share) as i64;
                    winners.push((winner.user_id(), payout));
                }

                LottoHandler::delete_tickets(&mut *tx).await?;

                let emojis = {
                    let data = ctx.data::<RwLock<Data>>();
                    let data = data.read().await;
                    data.emojis()
                };

                let coin = emojis
                    .emoji("heads")
                    .map_err(|n| GamblingError::Internal(format!("emoji '{n}' not in cache")))?;

                let mut lines = Vec::with_capacity(expected_winners);

                for (winner, payout) in winners {
                    GamblingHandler::add_coins(&mut *tx, winner, payout).await?;

                    let display_name = winner
                        .to_user(&ctx)
                        .await
                        .map(|u| u.display_name().to_string())
                        .unwrap_or_default();

                    let line = format!(
                        "{} ({display_name}) has won {} <:coin:{coin}> from the lottery!",
                        winner.mention(),
                        payout.format()
                    );

                    lines.push(line);
                }

                tx.commit().await?;

                let ticket_emoji = LOTTO_TICKET.emoji(&emojis).map_err(|e| {
                    GamblingError::Internal(format!("lotto ticket emoji failed: {e}"))
                })?;

                let embed = CreateEmbed::new()
                    .title(format!(
                        "<:coin:{coin}> <:coin:{coin}> Lottery!! <:coin:{coin}> <:coin:{coin}>"
                    ))
                    .field(
                        "Tickets Bought",
                        format!("{} {ticket_emoji}", total_tickets.format()),
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
