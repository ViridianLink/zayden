use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serenity::all::UserId;
use sqlx::{Database, FromRow};

use crate::ShopItem;

#[async_trait]
pub trait InventoryManager<Db: Database> {
    async fn item(
        conn: &mut Db::Connection,
        user_id: impl Into<UserId> + Send,
        item_id: &str,
    ) -> sqlx::Result<Option<InventoryRow>>;
}

#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct InventoryRow {
    pub id: i32,
    pub user_id: i64,
    pub item_id: String,
    pub quantity: i64,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize, FromRow)]
pub struct GamblingItem {
    pub item_id: String,
    pub quantity: i64,
}

impl From<&ShopItem<'_>> for GamblingItem {
    fn from(value: &ShopItem<'_>) -> Self {
        Self {
            quantity: 0,
            item_id: value.id.to_string(),
        }
    }
}
