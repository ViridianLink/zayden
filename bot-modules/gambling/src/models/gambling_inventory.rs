use serde::{Deserialize, Serialize};
use sqlx::FromRow;

use crate::{ItemInventory, ShopItem};

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

impl ItemInventory for GamblingItems {
    fn inventory(&self) -> &[GamblingItem] {
        &self.0
    }

    fn inventory_mut(&mut self) -> &mut Vec<GamblingItem> {
        &mut self.0
    }
}
