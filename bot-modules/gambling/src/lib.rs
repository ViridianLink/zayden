use std::collections::HashMap;
use std::sync::OnceLock;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serenity::all::{EmojiId, Http, UserId};
use tokio::sync::OnceCell;
use zayden_core::EmojiCache;

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
pub use common::{
    SHOP_ITEMS,
    ShopCurrency,
    ShopItem,
    ShopItems,
    ShopManager,
    ShopPage,
    ShopRow,
    shop,
};
pub use ctx_data::GamblingData;
pub use error::{GamblingError, Result};
pub use game_cache::GameCache;
pub use games::{HigherLower, Lotto, LottoManager, LottoRow, jackpot};
pub use goals::GoalHandler;
pub use models::{
    Coins,
    EffectsManager,
    EffectsRow,
    GamblingGoalsRow,
    GamblingItem,
    GamblingItems,
    GamblingManager,
    GameManager,
    GameRow,
    Gems,
    ItemInventory,
    MaxBet,
    MaxValues,
    MineHourly,
    Mining,
    Prestige,
    Stamina,
    StatsManager,
};
pub use stamina::{StaminaCron, StaminaManager};

const START_AMOUNT: i64 = 1000;
const GEM: char = '💎';

pub static CARD_DECK: OnceLock<Vec<EmojiId>> = OnceLock::new();

pub fn card_deck(emojis: &EmojiCache) -> Result<Vec<EmojiId>> {
    const SUITS: [&str; 4] = ["clubs", "diamonds", "hearts", "spades"];
    const VALUES: [&str; 13] =
        ["A", "02", "03", "04", "05", "06", "07", "08", "09", "10", "J", "Q", "K"];

    SUITS
        .iter()
        .flat_map(|suit| VALUES.iter().map(move |value| format!("{suit}_{value}")))
        .map(|name| {
            emojis.emoji(&name).map_err(|n| {
                GamblingError::Internal(format!("card emoji '{n}' not in cache"))
            })
        })
        .collect()
}

pub static CARD_TO_NUM: OnceLock<HashMap<EmojiId, u8>> = OnceLock::new();

pub fn card_to_num(emojis: &EmojiCache) -> Result<HashMap<EmojiId, u8>> {
    let deck = if let Some(deck) = CARD_DECK.get() {
        deck
    } else {
        let new_deck = card_deck(emojis)?;
        let _ = CARD_DECK.set(new_deck);
        CARD_DECK.get().ok_or_else(|| {
            GamblingError::Internal("CARD_DECK initialisation failed".to_string())
        })?
    };
    Ok(deck.iter().copied().zip((1u8..=13).cycle().take(52)).collect())
}

static BOT_ID: OnceCell<UserId> = OnceCell::const_new();

pub async fn bot_id(http: &Http) -> Result<UserId> {
    BOT_ID
        .get_or_try_init(|| async {
            http.get_current_user()
                .await
                .map(|user| user.id)
                .map_err(GamblingError::Serenity)
        })
        .await
        .copied()
}

pub fn tomorrow(now: Option<Timestamp>) -> Result<i64> {
    let ts = now
        .unwrap_or_else(Timestamp::now)
        .to_zoned(TimeZone::UTC)
        .date()
        .tomorrow()
        .map_err(|e| GamblingError::Internal(format!("date overflow: {e}")))?
        .at(0, 0, 0, 0)
        .to_zoned(TimeZone::UTC)
        .map_err(|e| {
            GamblingError::Internal(format!("timezone mapping failed: {e}"))
        })?
        .timestamp()
        .as_second();

    Ok(ts)
}

pub struct Leaderboard;
