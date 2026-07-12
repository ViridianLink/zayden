use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, DestinyError>;

#[derive(Debug, thiserror::Error)]
pub enum DestinyError {
    #[error(transparent)]
    Discord(#[from] serenity::Error),
    #[error(transparent)]
    BungieApi(#[from] bungie_api::BungieApiError),
    #[error(transparent)]
    GoogleSheets(#[from] google_sheets_api::Error),
    #[error(transparent)]
    HandlerError(HandlerError),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    ZaydenCore(#[from] zayden_core::CoreError),
    #[error("No perk found for: {0}")]
    PerkNotFound(String),
}

impl Respond for DestinyError {}

impl From<HandlerError> for DestinyError {
    fn from(e: HandlerError) -> Self {
        match e {
            HandlerError::Discord(e) => Self::Discord(e),
            e @ (HandlerError::Database(_) | HandlerError::Module { .. }) => {
                Self::HandlerError(e)
            },
        }
    }
}

impl From<DestinyError> for HandlerError {
    fn from(e: DestinyError) -> Self {
        Self::from_respond(e)
    }
}
