pub type Result<T> = std::result::Result<T, DeadlockError>;

#[derive(Debug, thiserror::Error)]
pub enum DeadlockError {
    #[error(transparent)]
    Api(#[from] deadlock_api::DeadlockApiError),
    #[error("Unknown hero `{0}`.")]
    UnknownHero(String),
    #[error("failed to decode cached deadlock-api data for {key}: {source}")]
    CacheDecode {
        key: String,
        #[source]
        source: serde_json::Error,
    },
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
}
