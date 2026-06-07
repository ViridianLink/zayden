use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, DestinyError>;

#[derive(Debug, thiserror::Error)]
pub enum DestinyError {
    #[error(transparent)]
    Discord(#[from] serenity::Error),
    #[error(transparent)]
    BungieApi(#[from] bungie_api::Error),
    #[error(transparent)]
    GoogleSheets(#[from] google_sheets_api::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("missing data: {0}")]
    MissingData(&'static str),
}

impl Respond for DestinyError {}

impl From<DestinyError> for HandlerError {
    fn from(e: DestinyError) -> Self {
        Self::from_respond(e)
    }
}
