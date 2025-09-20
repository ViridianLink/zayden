use std::collections::HashMap;

use async_trait::async_trait;
use chrono::NaiveDateTime;
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

    async fn remove_effect(conn: &mut Db::Connection, id: i32) -> sqlx::Result<Db::QueryResult>;

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

        let mut tx = pool.begin().await.unwrap();

        match Self::get_effect(&mut *tx, user_id, ALL_INS.id)
            .await
            .unwrap()
        {
            Some(effect) => {
                Self::remove_effect(&mut *tx, effect.id).await?;
            }
            None => {
                let max = GamblingHandler::max_bet(&mut *tx, user_id).await.unwrap();
                if bet > max {
                    return Err(Error::MaximumBetAmount(max));
                }
            }
        };

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

        let mut tx = pool.begin().await.unwrap();
        let mut effects = Self::get_effects(&mut *tx, user_id).await.unwrap();

        {
            let lucky_chip = effects.remove(LUCKY_CHIP.id);
            if let Some(id) = lucky_chip {
                Self::remove_effect(&mut *tx, id).await.unwrap();

                if win == Some(false) {
                    payout = bet;
                }
            }
        }

        for (item_id, id) in effects.drain() {
            Self::remove_effect(&mut *tx, id).await.unwrap();

            let item = SHOP_ITEMS.get(&item_id).unwrap();

            if win == Some(true) && item_id.starts_with("payout") {
                payout += (item.effect_fn)(bet, base_payout);
                continue;
            }
        }

        tx.commit().await.unwrap();

        payout.max(base_payout)
    }
}

pub struct EffectsRow {
    pub id: i32,
    pub item_id: String,
    pub expiry: Option<NaiveDateTime>,
}
