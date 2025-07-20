mod dispatch;

pub use dispatch::Dispatch;
use serenity::all::UserId;

use crate::{Coins, Gems, MaxBet};

pub trait EventRow: Coins + Gems + MaxBet + Send + Sync {}

impl<T: Coins + Gems + MaxBet + Send + Sync> EventRow for T {}

pub enum Event {
    Game(GameEvent),
    ShopPurchase(ShopPurchaseEvent),
    Send(SendEvent),
    Work(UserId),
}

impl Event {
    pub fn user_id(&self) -> UserId {
        match self {
            Self::Game(event) => event.user_id,
            Self::Work(id) => *id,
            Self::Send(event) => event.sender,
            Self::ShopPurchase(event) => event.user_id,
        }
    }
}

pub struct GameEvent {
    pub game_id: String,
    pub user_id: UserId,
    pub bet: i64,
    pub payout: i64,
    pub win: bool,
}

impl GameEvent {
    pub fn new(
        id: impl Into<String>,
        user_id: impl Into<UserId>,
        bet: i64,
        payout: i64,
        win: bool,
    ) -> Self {
        Self {
            game_id: id.into(),
            user_id: user_id.into(),
            bet,
            payout,
            win,
        }
    }
}

pub struct ShopPurchaseEvent {
    pub user_id: UserId,
    pub item_id: String,
}

impl ShopPurchaseEvent {
    pub fn new(user_id: impl Into<UserId>, item_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            item_id: item_id.into(),
        }
    }
}

pub struct SendEvent {
    pub amount: i64,
    pub sender: UserId,
}

impl SendEvent {
    pub fn new(amount: i64, sender: impl Into<UserId>) -> Self {
        Self {
            amount,
            sender: sender.into(),
        }
    }
}
