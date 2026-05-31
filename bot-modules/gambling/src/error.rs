use std::borrow::Cow;

use jiff::Timestamp;
use zayden_core::error::Respond;
use zayden_core::{Error as ZaydenError, FormatNum};

use crate::ShopCurrency;

pub type Result<T> = std::result::Result<T, GamblingError>;

#[derive(Debug)]
pub enum GamblingError {
    Overflow(i64),
    MessageConflict,

    PremiumRequired,
    InsufficientFunds { required: i64, currency: ShopCurrency },
    MinimumBetAmount(i64),
    MaximumBetAmount(i64),
    MaximumSendAmount(i64),
    DailyClaimed(i64),
    OutOfStamina(Timestamp),
    GiftUsed(i64),
    SelfGift,
    SelfSend,
    NegativeAmount,
    ZeroAmount,
    Cooldown(i64),
    InvalidPrediction,
    InvalidAmount,
    InsufficientCapacity(i64),
    ItemNotInInventory,
    InsufficientItemQuantity(i64),

    Serenity(serenity::Error),
    Sqlx(sqlx::Error),
}

impl std::fmt::Display for GamblingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Overflow(max) => {
                write!(f, "Overflow Error: Please enter a maximum of `{max}`")
            },
            Self::MessageConflict => ZaydenError::MessageConflict.fmt(f),
            Self::PremiumRequired => {
                write!(f, "Sorry, only supporters can use this option")
            },
            Self::InsufficientFunds { required, currency } => write!(
                f,
                "You do not have enough to make this.\nYou need the following resource: {} {currency:?}",
                required.format()
            ),
            Self::MinimumBetAmount(min) => {
                write!(f, "The minimum bet for this game is `{}`!", min.format())
            },
            Self::MaximumBetAmount(max) => {
                write!(f, "The maximum bet you've unlocked is `{}`!", max.format())
            },
            Self::MaximumSendAmount(max) => {
                write!(f, "The maximum you can send is `{}`!", max.format())
            },
            Self::DailyClaimed(timestamp) => {
                write!(f, "You collected today, try again <t:{timestamp}:R>")
            },
            Self::OutOfStamina(timestamp) => {
                write!(f, "You're out of stamina! Try again <t:{timestamp}:R>")
            },
            Self::GiftUsed(timestamp) => write!(
                f,
                "You can only gift someone once a day, try again <t:{timestamp}:R>",
            ),
            Self::SelfGift => {
                write!(f, "You can't give yourself a gift... How selfish!")
            },
            Self::SelfSend => write!(f, "You cannot send funds to yourself"),
            Self::NegativeAmount => write!(f, "Amount cannot be negative"),
            Self::ZeroAmount => write!(f, "Amount cannot be 0"),
            Self::Cooldown(timestamp) => {
                write!(f, "You are on a game cooldown. Try again <t:{timestamp}:R>")
            },
            Self::InvalidPrediction => write!(f, "Invalid prediction value."),
            Self::InvalidAmount => write!(f, "Invalid amount value."),
            Self::InsufficientCapacity(remaining) => write!(
                f,
                "You don't have enough capacity to buy that many.\nYou can buy `{remaining}` more before you are at capacity"
            ),
            Self::ItemNotInInventory => {
                write!(f, "You don't have that item in your inventory.")
            },
            Self::InsufficientItemQuantity(quantity) => write!(
                f,
                "Cannot sell that many. You only have {} of this item.",
                quantity.format()
            ),

            Self::Serenity(e) => write!(f, "serenity: {e:?}"),
            Self::Sqlx(e) => write!(f, "sqlx: {e:?}"),
        }
    }
}

impl std::error::Error for GamblingError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serenity(e) => Some(e),
            Self::Sqlx(e) => Some(e),
            Self::Overflow(_)
            | Self::MessageConflict
            | Self::PremiumRequired
            | Self::InsufficientFunds { .. }
            | Self::MinimumBetAmount(_)
            | Self::MaximumBetAmount(_)
            | Self::MaximumSendAmount(_)
            | Self::DailyClaimed(_)
            | Self::OutOfStamina(_)
            | Self::GiftUsed(_)
            | Self::SelfGift
            | Self::SelfSend
            | Self::NegativeAmount
            | Self::ZeroAmount
            | Self::Cooldown(_)
            | Self::InvalidPrediction
            | Self::InvalidAmount
            | Self::InsufficientCapacity(_)
            | Self::ItemNotInInventory
            | Self::InsufficientItemQuantity(_) => None,
        }
    }
}

impl Respond for GamblingError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Serenity(_) | Self::Sqlx(_) => None,
            Self::Overflow(_)
            | Self::MessageConflict
            | Self::PremiumRequired
            | Self::InsufficientFunds { .. }
            | Self::MinimumBetAmount(_)
            | Self::MaximumBetAmount(_)
            | Self::MaximumSendAmount(_)
            | Self::DailyClaimed(_)
            | Self::OutOfStamina(_)
            | Self::GiftUsed(_)
            | Self::SelfGift
            | Self::SelfSend
            | Self::NegativeAmount
            | Self::ZeroAmount
            | Self::Cooldown(_)
            | Self::InvalidPrediction
            | Self::InvalidAmount
            | Self::InsufficientCapacity(_)
            | Self::ItemNotInInventory
            | Self::InsufficientItemQuantity(_) => {
                Some(Cow::Owned(self.to_string()))
            },
        }
    }
}

impl From<serenity::Error> for GamblingError {
    fn from(value: serenity::Error) -> Self {
        Self::Serenity(value)
    }
}

impl From<sqlx::Error> for GamblingError {
    fn from(value: sqlx::Error) -> Self {
        Self::Sqlx(value)
    }
}
