use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, DestinyError>;

#[derive(Debug, thiserror::Error)]
pub enum DestinyError {
    #[error(transparent)]
    Discord(#[from] serenity::Error),
    #[error(transparent)]
    BungieApi(#[from] bungie_api::Error),
}

impl Respond for DestinyError {}

impl From<DestinyError> for HandlerError {
    fn from(e: DestinyError) -> Self {
        Self::from_respond(e)
    }
}
