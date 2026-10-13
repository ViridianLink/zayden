use std::env::VarError;
use std::io;
use std::net::AddrParseError;

use oauth2::url::ParseError;

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("DATABASE_URL is missing or not valid unicode: {0}")]
    DatabaseUrl(#[from] VarError),
    #[error("could not connect to the database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("could not load the bot config: {0}")]
    Config(#[from] zayden_app::Error),
    #[error("invalid Discord OAuth URL: {0}")]
    OAuthUrl(#[from] ParseError),
    #[error("invalid bind address: {0}")]
    BindAddr(#[from] AddrParseError),
    #[error("invalid DASHBOARD_COOKIE_KEY: {0}")]
    CookieKey(#[from] CookieKeyError),
    #[error(
        "could not load the asset bundle (run `topcoat asset bundle -p web`): {0}"
    )]
    AssetBundle(#[source] io::Error),
    #[error("could not listen on the bind address: {0}")]
    Listen(#[source] io::Error),
    #[error("server error: {0}")]
    Serve(#[source] io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum CookieKeyError {
    #[error("not standard base64: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("decodes to {0} bytes, at least 64 are needed")]
    TooShort(usize),
}
