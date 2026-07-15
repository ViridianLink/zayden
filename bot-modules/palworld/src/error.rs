use std::borrow::Cow;

use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, PalworldError>;

#[derive(Debug, thiserror::Error)]
pub enum PalworldError {
    #[error("Palworld data is temporarily unavailable. Please try again shortly.")]
    SourceUnavailable,
    #[error("Couldn't find `{query}` in {entity}.")]
    NotFound { entity: &'static str, query: String },
    #[error("`{0}` isn't a known Palworld element.")]
    UnknownElement(String),
    #[error(
        "No world save is loaded, so there are no rosters to read. Upload your \
         own with `/palworld upload`, or - on a shared/multiplayer world - ask \
         whoever hosts the world to upload its `Level.sav`. A co-op client's \
         `LocalData.sav` does not contain Pal data and can't be used."
    )]
    NoWorld,
    #[error(
        "You haven't linked a player. Use `/palworld link <name>`, pass a \
         `player:`, or `/palworld upload` your own `Level.sav`."
    )]
    NotLinked,
    #[error(
        "You can only link to a host you share a server with. Run `/palworld \
         link` from a server you're both in."
    )]
    LinkNotSameGuild,
    #[error(
        "That member hasn't uploaded a world yet. Ask them to run `/palworld \
         upload`, then link again."
    )]
    LinkHostNoWorld,
    #[error(
        "The world you linked to is no longer available (the host's upload \
         expired). Ask them to re-upload, or `/palworld unlink`."
    )]
    LinkedWorldGone,

    #[error("FlareSolverr error: {0}")]
    FlareSolverr(String),
    #[error("failed to parse source content: {0}")]
    Parse(String),

    #[error("failed to read world save: {0}")]
    Save(String),
    #[error("failed to parse GVAS save data: {0}")]
    Gvas(String),
    #[error("failed to refresh world save from the game server: {0}")]
    Pelican(String),
    #[error("save upload rejected: {0}")]
    Upload(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("network error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("discord error: {0}")]
    Serenity(#[from] serenity::Error),
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

impl Respond for PalworldError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::SourceUnavailable
            | Self::NotFound { .. }
            | Self::UnknownElement(_)
            | Self::NoWorld
            | Self::NotLinked
            | Self::LinkNotSameGuild
            | Self::LinkHostNoWorld
            | Self::LinkedWorldGone
            | Self::Upload(_) => Some(Cow::Owned(self.to_string())),
            Self::Save(_) | Self::Gvas(_) | Self::Io(_) => Some(Cow::Borrowed(
                "Couldn't read the world save. If it's your upload, re-upload a \
                 fresh `Level.sav` with `/palworld upload`.",
            )),
            Self::Pelican(_) => Some(Cow::Borrowed(
                "Couldn't reach the game server to refresh the world save.",
            )),
            Self::FlareSolverr(_)
            | Self::Parse(_)
            | Self::Reqwest(_)
            | Self::Serenity(_)
            | Self::Sqlx(_) => None,
        }
    }
}

impl From<PalworldError> for HandlerError {
    fn from(e: PalworldError) -> Self {
        Self::from_respond(e)
    }
}

impl From<HandlerError> for PalworldError {
    fn from(e: HandlerError) -> Self {
        match e {
            HandlerError::Discord(e) => Self::Serenity(e),
            HandlerError::Database(e) => Self::Sqlx(e),
            HandlerError::Module { source, .. } => Self::Parse(source.to_string()),
        }
    }
}
