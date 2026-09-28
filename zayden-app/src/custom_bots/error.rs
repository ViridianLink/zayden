use std::env::VarError;
use std::num::ParseIntError;
use std::string::FromUtf8Error;

use aws_lc_rs::error::Unspecified;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CustomBotError {
    #[error("{name} is not set")]
    MissingEnvVar {
        name: &'static str,
        #[source]
        source: VarError,
    },

    #[error("invalid custom bot key spec: {0}")]
    InvalidKeySpec(String),

    #[error("custom bot key id `{id}` is not a number")]
    KeyId {
        id: String,
        #[source]
        source: ParseIntError,
    },

    #[error("custom bot key {id} is not base64")]
    KeyNotBase64 {
        id: i16,
        #[source]
        source: base64::DecodeError,
    },

    #[error("custom bot key {id} must be 32 bytes")]
    KeyLength {
        id: i16,
        #[source]
        source: Unspecified,
    },

    #[error("no custom bot key with id {0} is loaded")]
    UnknownKey(i16),

    #[error("failed to encrypt the bot token")]
    Encrypt(#[source] Unspecified),

    #[error("the bot token failed to decrypt")]
    Decrypt(#[source] Unspecified),

    #[error("the stored nonce has the wrong length")]
    MalformedNonce(#[source] Unspecified),

    #[error("the decrypted bot token is not UTF-8")]
    NotUtf8(#[from] FromUtf8Error),

    #[error("guild {guild_id} is already served by custom bot {application_id}")]
    GuildTaken { guild_id: i64, application_id: i64 },

    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}
