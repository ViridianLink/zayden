use zayden_core::Error as ZaydenError;
use zayden_core::FormatNum;

use crate::ShopCurrency;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    Overflow(i64),
    MessageConflict,

    PremiumRequired,
    InsufficientFunds {
        required: i64,
        currency: ShopCurrency,
    },
    MinimumBetAmount(i64),
    MaximumBetAmount(i64),
    MaximumSendAmount(i64),
    DailyClaimed(i64),
    OutOfStamina(i64),
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

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Overflow(max) => write!(f, "Overflow Error: Please enter a maximum of `{max}`"),
            Error::MessageConflict => ZaydenError::MessageConflict.fmt(f),
            Error::PremiumRequired => write!(f, "Sorry, only supporters can use this option"),
            Error::InsufficientFunds { required, currency } => write!(
                f,
                "You do not have enough to make this.\nYou need the following resource: {} {currency:?}",
                required.format()
            ),
            Error::MinimumBetAmount(min) => {
                write!(f, "The minimum bet for this game is `{}`!", min.format())
            }
            Error::MaximumBetAmount(max) => {
                write!(f, "The maximum bet you've unlocked is `{}`!", max.format())
            }
            Error::MaximumSendAmount(max) => {
                write!(f, "The maximum you can send is `{}`!", max.format())
            }
            Error::DailyClaimed(timestamp) => {
                write!(f, "You collected today, try again <t:{timestamp}:R>",)
            }
            Error::OutOfStamina(timestamp) => {
                write!(f, "You're out of stamina! Try again <t:{timestamp}:R>")
            }
            Error::GiftUsed(timestamp) => write!(
                f,
                "You can only gift someone once a day, try again <t:{timestamp}:R>",
            ),
            Error::SelfGift => write!(f, "You can't give yourself a gift... How selfish!"),
            Error::SelfSend => write!(f, "You cannot send funds to yourself"),
            Error::NegativeAmount => write!(f, "Amount cannot be negative"),
            Error::ZeroAmount => write!(f, "Amount cannot be 0"),
            Error::Cooldown(timestamp) => {
                write!(f, "You are on a game cooldown. Try again <t:{timestamp}:R>")
            }
            Error::InvalidPrediction => write!(f, "Invalid prediction value."),
            Error::InvalidAmount => write!(f, "Invalid amount value."),
            Error::InsufficientCapacity(remaining) => write!(
                f,
                "You don't have enough capacity to buy that many.\nYou can buy `{remaining}` more before you are at capacity"
            ),
            Error::ItemNotInInventory => write!(f, "You don't have that item in your inventory."),
            Error::InsufficientItemQuantity(quantity) => write!(
                f,
                "Cannot sell that many. You only have {} of this item.",
                quantity.format()
            ),

            Error::Serenity(e) => unimplemented!("Unhandled Serenity error: {e:?}"),
            Error::Sqlx(e) => unimplemented!("Unhandled SQLx error: {e:?}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<zayden_core::Error> for Error {
    fn from(_value: zayden_core::Error) -> Self {
        Self::MessageConflict
    }
}

impl From<serenity::Error> for Error {
    fn from(value: serenity::Error) -> Self {
        Self::Serenity(value)
    }
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::Sqlx(value)
    }
}
