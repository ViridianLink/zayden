pub type Result<T> = std::result::Result<T, YoutubeError>;

#[derive(Debug, thiserror::Error)]
pub enum YoutubeError {
    #[error("YouTube rejected the request's credentials")]
    Unauthorized,
    #[error("that Google account has no YouTube channel")]
    NoChannel,
    #[error("YouTube returned an unexpected payload: {0}")]
    Payload(String),
    #[error("the WebSub hub refused the request with status {0}")]
    Hub(u16),

    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    #[error("internal error: {0}")]
    Internal(String),
}
