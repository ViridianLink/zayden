use crate::auth::AuthError;

#[derive(Debug, thiserror::Error)]
pub enum FlashError {
    #[error("the flash cookie needs the cookie layer: {0}")]
    CookieJar(#[from] AuthError),
    #[error("a flash message needs text")]
    EmptyMessage,
    #[error("no key is available to sign the flash cookie")]
    NoKey,
}
