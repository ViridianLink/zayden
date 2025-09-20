use std::{collections::HashMap, sync::OnceLock};

use chrono::{DateTime, Days, NaiveTime, Utc};
use serenity::all::{EmojiId, Http, UserId};

pub mod commands;
pub mod common;
pub mod components;
pub mod ctx_data;
pub mod error;
pub mod events;
pub mod game_cache;
pub mod games;
pub mod goals;
pub mod models;
pub mod stamina;
pub mod utils;

pub use commands::Commands;
pub use commands::goals::GoalsManager;
pub use common::{SHOP_ITEMS, ShopCurrency, ShopItem, ShopItems, ShopManager, ShopPage, ShopRow};
pub use ctx_data::GamblingData;
pub use error::Error;
use error::Result;
pub use game_cache::GameCache;
pub use games::{HigherLower, Lotto, LottoManager, LottoRow, jackpot};
pub use goals::GoalHandler;
pub use models::{
    Coins, EffectsManager, EffectsRow, GamblingGoalsRow, GamblingItem, GamblingManager,
    GameManager, GameRow, Gems, ItemInventory, MaxBet, MaxValues, MineHourly, Mining, Prestige,
    Stamina, StatsManager,
};
pub use stamina::{StaminaCron, StaminaManager};
use tokio::sync::OnceCell;
use zayden_core::EmojiCache;

const START_AMOUNT: i64 = 1000;
const GEM: char = '💎';

pub static CARD_DECK: OnceLock<Vec<EmojiId>> = OnceLock::new();

pub fn card_deck(emojis: &EmojiCache) -> Vec<EmojiId> {
    const SUITS: [&str; 4] = ["clubs", "diamonds", "hearts", "spades"];
    const VALUES: [&str; 13] = [
        "A", "02", "03", "04", "05", "06", "07", "08", "09", "10", "J", "Q", "K",
    ];

    let emoji_names: Vec<String> = SUITS
        .iter()
        .flat_map(|suit| VALUES.iter().map(move |value| format!("{suit}_{value}")))
        .collect();

    emoji_names
        .into_iter()
        .map(|name| emojis.emoji(&name).expect("Emoji doesn't exist on Zayden"))
        .collect()
}

pub static CARD_TO_NUM: OnceLock<HashMap<EmojiId, u8>> = OnceLock::new();

fn card_to_num(emojis: &EmojiCache) -> HashMap<EmojiId, u8> {
    CARD_DECK
        .get_or_init(|| card_deck(emojis))
        .iter()
        .copied()
        .zip((1u8..=13).cycle().take(52))
        .collect()
}

static BOT_ID: OnceCell<UserId> = OnceCell::const_new();

pub async fn bot_id(http: &Http) -> UserId {
    *BOT_ID
        .get_or_try_init(|| async { http.get_current_user().await.map(|user| user.id) })
        .await
        .unwrap()
}

fn tomorrow(now: Option<DateTime<Utc>>) -> i64 {
    now.unwrap_or_else(Utc::now)
        .checked_add_days(Days::new(1))
        .unwrap()
        .with_time(NaiveTime::MIN)
        .unwrap()
        .timestamp()
}
