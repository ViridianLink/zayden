use std::borrow::Cow;

use zayden_core::error::{HandlerError, Respond};

use crate::loadouts::{ArmourSlot, StatKind};

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
    #[error("You need the Manage Server permission to use this command.")]
    NotPrivileged,
    #[error("This command can only be used in the Zayden support server.")]
    NotHomeGuild,
}

impl Respond for DestinyError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::NotPrivileged | Self::NotHomeGuild => {
                Some(Cow::Owned(self.to_string()))
            },
            Self::Discord(_)
            | Self::BungieApi(_)
            | Self::GoogleSheets(_)
            | Self::HandlerError(_)
            | Self::Sqlx(_)
            | Self::ZaydenCore(_)
            | Self::PerkNotFound(_) => None,
        }
    }
}

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

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum DraftError {
    #[error("{field} is required")]
    Required { field: &'static str },
    #[error("{field} must be at most {max} characters")]
    TooLong { field: &'static str, max: usize },
    #[error("{field} allows at most {max} entries")]
    TooMany { field: &'static str, max: usize },
    #[error(
        "`{0}` is not a valid emoji name (2-32 lowercase letters, digits or underscores)"
    )]
    InvalidEmojiKey(String),
    #[error("tag `{0}` repeats another button on the loadout")]
    DuplicateTag(String),
    #[error("armour slot {0} appears more than once")]
    DuplicateArmourSlot(ArmourSlot),
    #[error("stat {0} appears more than once")]
    DuplicateStat(StatKind),
    #[error("stat {stat} must be between 0 and {max}, got {value}")]
    StatOutOfRange { stat: StatKind, value: i16, max: i16 },
    #[error("{field} must be an https:// link")]
    NotHttps { field: &'static str },
    #[error(
        "this build needs {needed} discord components, the limit is {max}; remove a \
         weapon, armour piece or tag"
    )]
    TooManyComponents { needed: usize, max: usize },
    #[error(
        "this build may render up to {estimate} characters of text, discord's limit \
         is {max}; shorten how it works or remove some perks, mods or fragments"
    )]
    TooMuchText { estimate: usize, max: usize },
}

#[derive(Debug, thiserror::Error)]
pub enum SaveError {
    #[error("a loadout with this class, element and name already exists")]
    DuplicateName,
    #[error("loadout {0} does not exist")]
    NotFound(i32),
    #[error(transparent)]
    Sqlx(sqlx::Error),
}

impl From<sqlx::Error> for SaveError {
    fn from(e: sqlx::Error) -> Self {
        if e.as_database_error().is_some_and(|d| {
            d.constraint() == Some("destiny2_loadouts_class_element_name_key")
        }) {
            Self::DuplicateName
        } else {
            Self::Sqlx(e)
        }
    }
}
