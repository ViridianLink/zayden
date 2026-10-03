use destiny2::{DraftError, SaveError};
use twilight_http::response::DeserializeBodyError;

use crate::auth::AuthError;

#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Form(#[from] LoadoutFormError),
    #[error(transparent)]
    Draft(#[from] DraftError),
    #[error(transparent)]
    Save(#[from] SaveError),
    #[error(transparent)]
    Emoji(#[from] EmojiUploadError),
    #[error("loadout {0} does not exist")]
    LoadoutNotFound(i32),
    #[error("zayden_id is not configured")]
    ZaydenIdNotConfigured,
    #[error("{0}")]
    Database(String),
    #[error("{0}")]
    Discord(String),
}

impl AdminError {
    #[must_use]
    pub const fn is_denied(&self) -> bool {
        matches!(self, Self::Auth(e) if e.is_denied())
    }
}

impl From<sqlx::Error> for AdminError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e.to_string())
    }
}

impl From<twilight_http::Error> for AdminError {
    fn from(e: twilight_http::Error) -> Self {
        Self::Discord(e.to_string())
    }
}

impl From<DeserializeBodyError> for AdminError {
    fn from(e: DeserializeBodyError) -> Self {
        Self::Discord(e.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LoadoutFormError {
    #[error("unknown {field} `{value}`")]
    UnknownOption { field: &'static str, value: String },
    #[error("stat value `{0}` is not a whole number")]
    StatValue(String),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LoadoutFieldError {
    #[error("unknown field `{0}`")]
    UnknownField(String),
    #[error("field `{0}` appears more than once")]
    RepeatedField(String),
    #[error("loadout id `{0}` is not a whole number")]
    InvalidId(String),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EmojiUploadError {
    #[error("emoji names are 2-32 lowercase letters, digits or underscores")]
    InvalidName,
    #[error("`{0}` is reserved for a built-in class, element, weapon or stat icon")]
    ReservedName(String),
    #[error("Zayden already has an emoji named `{0}`")]
    NameTaken(String),
    #[error("the image link must be an https:// URL")]
    NotHttps,
    #[error("that link points at a private network address")]
    PrivateAddress,
    #[error("couldn't download the image: {0}")]
    Fetch(String),
    #[error("images must be 256 KiB or smaller")]
    TooLarge,
    #[error("images must be PNG, JPEG, GIF or WebP")]
    UnsupportedType,
    #[error("the chosen file couldn't be read")]
    BadDataUri,
    #[error("Discord refused the emoji: {0}")]
    Discord(String),
}
