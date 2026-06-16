use std::borrow::Cow;

use zayden_core::CoreError as ZaydenError;
use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, EndgameAnalysisError>;

#[derive(Debug, thiserror::Error)]
pub enum EndgameAnalysisError {
    WeaponNotFound(String),
    MissingHeaderRow(String),

    Io(#[from] std::io::Error),
    Json(#[from] serde_json::Error),
    BungieApi(#[from] bungie_api::BungieApiError),
    GoogleSheets(#[from] google_sheets_api::Error),
    ZaydenCore(#[from] ZaydenError),
}

impl std::fmt::Display for EndgameAnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WeaponNotFound(weapon) => write!(f, "Weapon {weapon} not found"),
            Self::MissingHeaderRow(sheet) => write!(f, "Sheet '{sheet}' has no header row"),
            Self::Io(e) => e.fmt(f),
            Self::Json(e) => e.fmt(f),
            Self::BungieApi(e) => e.fmt(f),
            Self::GoogleSheets(e) => e.fmt(f),
            Self::ZaydenCore(e) => e.fmt(f),
        }
    }
}

impl Respond for EndgameAnalysisError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::WeaponNotFound(_) => Some(Cow::Owned(self.to_string())),
            Self::ZaydenCore(e) => e.user_message(),
            Self::Io(_)
            | Self::Json(_)
            | Self::BungieApi(_)
            | Self::GoogleSheets(_)
            | Self::MissingHeaderRow(_) => None,
        }
    }
}

impl From<HandlerError> for EndgameAnalysisError {
    fn from(e: HandlerError) -> Self {
        match e {
            HandlerError::Discord(e) => Self::ZaydenCore(ZaydenError::Serenity(e)),
            HandlerError::Database(e) => Self::ZaydenCore(ZaydenError::Sqlx(e)),
            HandlerError::Module { source, .. } => {
                Self::ZaydenCore(ZaydenError::Other(source.to_string()))
            },
        }
    }
}

impl From<EndgameAnalysisError> for HandlerError {
    fn from(e: EndgameAnalysisError) -> Self {
        Self::from_respond(e)
    }
}

impl From<serenity::Error> for EndgameAnalysisError {
    fn from(value: serenity::Error) -> Self {
        Self::ZaydenCore(zayden_core::CoreError::Serenity(value))
    }
}
