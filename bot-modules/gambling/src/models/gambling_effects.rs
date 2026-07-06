use std::collections::HashMap;

use async_trait::async_trait;
use futures::TryStreamExt;
use jiff_sqlx::Timestamp;
use serenity::all::UserId;
use sqlx::postgres::PgQueryResult;
use sqlx::postgres::types::PgInterval;
use sqlx::{Database, PgConnection, Pool, Postgres};
use zayden_core::as_i64;

use crate::models::effects::get_effect;
use crate::models::gambling::GamblingManager;
use crate::shop::{ALL_INS, ShopItem};
use crate::{GamblingError, Result, ShopCurrency};

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
            return Err(GamblingError::MinimumBetAmount(MIN));
        }

        if bet > coins {
            return Err(GamblingError::InsufficientFunds {
                required: bet - coins,
                currency: ShopCurrency::Coins,
            });
        }

        let user_id = user_id.into();

        let mut conn = pool.acquire().await?;

        let max = GamblingHandler::max_bet(&mut *conn, user_id).await?;

        if bet > max {
            let all_in = bet == coins;

            let all_ins_active = all_in
                && Self::get_effect(&mut *conn, user_id, ALL_INS.id)
                    .await?
                    .and_then(|row| row.expiry)
                    .is_some_and(|expiry| expiry.to_jiff() > jiff::Timestamp::now());

            if !all_ins_active {
                return Err(GamblingError::MaximumBetAmount(max));
            }
        }

        Ok(())
    }

    async fn payout(
        pool: &Pool<Db>,
        user_id: impl Into<UserId> + Send,
        bet: i64,
        payout: i64,
        win: Option<bool>,
    ) -> i64 {
        let base_payout = payout;

        let Some(win) = win else {
            return base_payout;
        };

        let user_id = user_id.into();

        let result: sqlx::Result<i64> = (async {
            let mut tx = pool.begin().await?;
            let effects = Self::get_effects(&mut *tx, user_id).await?;

            let mut contribution: i64 = 0;

            for (item_id, id) in effects {
                Self::remove_effect(&mut *tx, id).await?;

                let Some(effect) = get_effect(&item_id) else {
                    tracing::warn!(
                        "effect item_id '{item_id}' not found in registry, skipping"
                    );
                    continue;
                };

                contribution += if win {
                    effect.on_win(bet, base_payout)
                } else {
                    effect.on_loss(bet, base_payout)
                };
            }

            tx.commit().await?;

            let payout = if win {
                bet + (base_payout - bet).max(contribution)
            } else {
                base_payout.max(contribution)
            };

            Ok(payout)
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

pub struct EffectsTable;

#[async_trait]
impl EffectsManager<Postgres> for EffectsTable {
    async fn get_effects(
        conn: &mut PgConnection,
        user_id: impl Into<UserId> + Send,
    ) -> sqlx::Result<HashMap<String, i32>> {
        let user_id = user_id.into();

        sqlx::query_as!(
            EffectsRow,
            r#"SELECT DISTINCT ON (item_id) id, item_id, expiry as "expiry: jiff_sqlx::Timestamp" FROM gambling_effects WHERE user_id = $1"#,
            as_i64(user_id.get()),
        )
        .fetch(conn)
        .map_ok(|row| (row.item_id, row.id))
        .try_collect()
        .await
    }

    async fn get_effect(
        conn: &mut PgConnection,
        user_id: impl Into<UserId> + Send,
        effect: &str,
    ) -> sqlx::Result<Option<EffectsRow>> {
        let user_id = user_id.into();

        sqlx::query_as!(
            EffectsRow,
            r#"SELECT DISTINCT ON (item_id) id, item_id, expiry as "expiry: jiff_sqlx::Timestamp" FROM gambling_effects WHERE user_id = $1 AND item_id = $2"#,
            as_i64(user_id.get()),
            effect
        )
        .fetch_optional(conn)
        .await
    }

    async fn add_effect(
        conn: &mut PgConnection,
        user_id: impl Into<UserId> + Send,
        item: &ShopItem<'_>,
    ) -> sqlx::Result<PgQueryResult> {
        let user_id = user_id.into();

        let duration = item
            .effect_duration
            .map(|d| {
                PgInterval::try_from(d)
                    .map_err(|e| sqlx::Error::Protocol(e.to_string()))
            })
            .transpose()?;

        sqlx::query!(
            "INSERT INTO gambling_effects (user_id, item_id, expiry)
            VALUES ($1, $2, NOW() + $3)
            ON CONFLICT (user_id, item_id)
            DO UPDATE SET
                expiry = GREATEST(gambling_effects.expiry + $3, EXCLUDED.expiry)",
            as_i64(user_id.get()),
            item.id,
            duration
        )
        .execute(conn)
        .await
    }

    async fn remove_effect(
        conn: &mut PgConnection,
        id: i32,
    ) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "DELETE FROM gambling_effects WHERE id = $1 AND (expiry <= NOW() OR expiry IS NULL)",
            id
        )
        .execute(conn)
        .await
    }
}
