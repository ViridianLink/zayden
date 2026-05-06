use std::collections::HashMap;

use async_trait::async_trait;
use futures::TryStreamExt;
use gambling::{EffectsManager, EffectsRow, shop::ShopItem};
use serenity::all::UserId;
use sqlx::{
    PgConnection, Postgres,
    postgres::{PgQueryResult, types::PgInterval},
};

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
            user_id.get() as i64,
        )
        .fetch(conn).map_ok(|row| (row.item_id, row.id)).try_collect()
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
            user_id.get() as i64,
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
            .map(|d| PgInterval::try_from(d).unwrap());

        sqlx::query!(
            "INSERT INTO gambling_effects (user_id, item_id, expiry)
            VALUES ($1, $2, NOW() + $3)
            ON CONFLICT (user_id, item_id)
            DO UPDATE SET
                expiry = GREATEST(gambling_effects.expiry + $3, EXCLUDED.expiry)",
            user_id.get() as i64,
            item.id,
            duration
        )
        .execute(conn)
        .await
    }

    async fn remove_effect(conn: &mut PgConnection, id: i32) -> sqlx::Result<PgQueryResult> {
        sqlx::query!(
            "DELETE FROM gambling_effects WHERE id = $1 AND (expiry <= NOW() OR expiry IS NULL)",
            id
        )
        .execute(conn)
        .await
    }
}
