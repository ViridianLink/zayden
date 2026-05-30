use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serenity::all::UserId;
use sqlx::{Database, FromRow};

use crate::shop::LOTTO_TICKET;
use crate::{ItemInventory, SHOP_ITEMS, ShopItem};

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
        Self { quantity: 0, item_id: value.id.to_string() }
    }
}

pub struct GamblingItems(pub Vec<GamblingItem>);

impl GamblingItems {
    pub fn do_prestige(&mut self) {
        self.0.retain(|item| {
            let is_sellable = SHOP_ITEMS
                .get(&item.item_id)
                .is_some_and(|shop_item_data| shop_item_data.sellable);

            item.item_id != LOTTO_TICKET.id && !is_sellable
        });
    }
}
impl ItemInventory for GamblingItems {
    fn inventory(&self) -> &[GamblingItem] {
        &self.0
    }

    fn inventory_mut(&mut self) -> &mut Vec<GamblingItem> {
        &mut self.0
    }
}
