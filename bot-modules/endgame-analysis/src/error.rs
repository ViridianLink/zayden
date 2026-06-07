use std::borrow::Cow;

use zayden_core::Error as ZaydenError;
use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, EndgameAnalysisError>;

#[derive(Debug)]
pub enum EndgameAnalysisError {
    WeaponNotFound(String),
    MissingData(&'static str),

    Io(std::io::Error),
    Json(serde_json::Error),
    BungieApi(bungie_api::Error),
    GoogleSheets(google_sheets_api::Error),
    ZaydenCore(ZaydenError),
}

impl std::fmt::Display for EndgameAnalysisError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WeaponNotFound(weapon) => write!(f, "Weapon {weapon} not found"),
            Self::MissingData(ctx) => write!(f, "Missing spreadsheet data: {ctx}"),
            Self::Io(e) => e.fmt(f),
            Self::Json(e) => e.fmt(f),
            Self::BungieApi(e) => e.fmt(f),
            Self::GoogleSheets(e) => e.fmt(f),
            Self::ZaydenCore(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for EndgameAnalysisError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Json(e) => Some(e),
            Self::BungieApi(e) => Some(e),
            Self::GoogleSheets(e) => Some(e),
            Self::ZaydenCore(e) => Some(e),
            Self::WeaponNotFound(_) | Self::MissingData(_) => None,
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
            | Self::MissingData(_) => None,
        }
    }
}

impl From<serenity::Error> for EndgameAnalysisError {
    fn from(value: serenity::Error) -> Self {
        Self::ZaydenCore(ZaydenError::Serenity(value))
    }
}

impl From<std::io::Error> for EndgameAnalysisError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for EndgameAnalysisError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

impl From<bungie_api::Error> for EndgameAnalysisError {
    fn from(e: bungie_api::Error) -> Self {
        Self::BungieApi(e)
    }
}

impl From<google_sheets_api::Error> for EndgameAnalysisError {
    fn from(e: google_sheets_api::Error) -> Self {
        Self::GoogleSheets(e)
    }
}

impl From<EndgameAnalysisError> for HandlerError {
    fn from(e: EndgameAnalysisError) -> Self {
        Self::from_respond(e)
    }
}
