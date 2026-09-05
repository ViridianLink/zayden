pub type Result<T> = std::result::Result<T, PatreonError>;

#[derive(Debug, thiserror::Error)]
pub enum PatreonError {
    #[error("Patreon rejected the stored authorisation")]
    Unauthorized,
    #[error(
        "that Patreon account has no campaign; only a creator account can be \
         connected, not a patron account"
    )]
    NoCampaign,
    #[error("Patreon returned an unexpected payload: {0}")]
    Payload(String),

    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("internal error: {0}")]
    Internal(String),
}
