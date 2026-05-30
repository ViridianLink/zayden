use std::collections::HashMap;

use async_trait::async_trait;
use jiff_sqlx::Timestamp;
use serenity::all::UserId;
use sqlx::{Database, Pool};

use crate::common::shop::{ALL_INS, LUCKY_CHIP, SHOP_ITEMS, ShopItem};
use crate::models::gambling::GamblingManager;
use crate::{Error, Result, ShopCurrency};

#[async_trait]
pub trait EffectsManager<Db: Database>: Send {
    async fn get_effects(
        conn: &mut Db::Connection,
        user_id: impl Into<UserId> + Send,
    ) -> sqlx::Result<HashMap<String, i32>>;

    async fn get_effect(
        conn: &mut Db::Connection,
        user_id: impl Into<UserId> + Send,
        effect: &str,
    ) -> sqlx::Result<Option<EffectsRow>>;

    async fn add_effect(
        conn: &mut Db::Connection,
        user_id: impl Into<UserId> + Send,
        item: &ShopItem<'_>,
    ) -> sqlx::Result<Db::QueryResult>;

    async fn remove_effect(
        conn: &mut Db::Connection,
        id: i32,
    ) -> sqlx::Result<Db::QueryResult>;

    async fn bet_limit<GamblingHandler: GamblingManager<Db>>(
        pool: &Pool<Db>,
        user_id: impl Into<UserId> + Send,
        bet: i64,
        coins: i64,
    ) -> Result<()> {
        const MIN: i64 = 1;

        if bet < MIN {
            return Err(Error::MinimumBetAmount(MIN));
        }

        let user_id = user_id.into();

        let mut tx = pool.begin().await?;

        if let Some(effect) = Self::get_effect(&mut *tx, user_id, ALL_INS.id)
            .await
            .expect("async call")
        {
            Self::remove_effect(&mut *tx, effect.id).await?;
        } else {
            let max = GamblingHandler::max_bet(&mut *tx, user_id).await?;
            if bet > max {
                return Err(Error::MaximumBetAmount(max));
            }
        }

        tx.commit().await?;

        if bet > coins {
            return Err(Error::InsufficientFunds {
                required: bet - coins,
                currency: ShopCurrency::Coins,
            });
        }

        Ok(())
    }

    async fn payout(
        pool: &Pool<Db>,
        user_id: impl Into<UserId> + Send,
        bet: i64,
        mut payout: i64,
        win: Option<bool>,
    ) -> i64 {
        let base_payout = payout;
        payout = 0;

        let user_id = user_id.into();

        let result: sqlx::Result<i64> = (async {
            let mut tx = pool.begin().await?;
            let mut effects = Self::get_effects(&mut *tx, user_id).await?;

            {
                let lucky_chip = effects.remove(LUCKY_CHIP.id);
                if let Some(id) = lucky_chip {
                    Self::remove_effect(&mut *tx, id).await?;

                    if win == Some(false) {
                        payout = bet;
                    }
                }
            }

            for (item_id, id) in effects.drain() {
                Self::remove_effect(&mut *tx, id).await?;

                let item = SHOP_ITEMS
                    .get(&item_id)
                    .expect("effect item_id is always a valid SHOP_ITEMS key");

                if win == Some(true) && item_id.starts_with("payout") {
                    payout += (item.effect_fn)(bet, base_payout);
                }
            }

            tx.commit().await?;

            Ok(payout.max(base_payout))
        })
        .await;

        result.unwrap_or_else(|e| {
            tracing::error!(error = ?e, "payout effects DB error, falling back to base payout");
            base_payout
        })
    }
}

pub struct EffectsRow {
    pub id: i32,
    pub item_id: String,
    pub expiry: Option<Timestamp>,
}
