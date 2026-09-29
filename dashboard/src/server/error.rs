use leptos::prelude::ServerFnError;

pub(crate) const UNAUTHENTICATED: &str = "unauthenticated";
pub(crate) const FORBIDDEN: &str = "forbidden";

/// Whether the server refused the caller, as opposed to failing to serve them.
pub(crate) fn is_denied(e: &ServerFnError) -> bool {
    let ServerFnError::ServerError(msg) = e else {
        return false;
    };

    matches!(msg.as_str(), UNAUTHENTICATED | FORBIDDEN)
}

#[cfg(feature = "ssr")]
#[derive(Debug, thiserror::Error)]
pub enum ForeignIdError {
    #[error("that channel is not in this server")]
    Channel,
    #[error("that role is not in this server")]
    Role,
}

#[cfg(feature = "ssr")]
#[derive(Debug, thiserror::Error)]
pub enum LoadoutFormError {
    #[error("unknown {field} `{value}`")]
    UnknownOption { field: &'static str, value: String },
    #[error("stat value `{0}` is not a whole number")]
    StatValue(String),
}

#[cfg(feature = "ssr")]
#[derive(Debug, thiserror::Error)]
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
