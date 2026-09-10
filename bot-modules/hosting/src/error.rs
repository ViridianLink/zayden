use std::borrow::Cow;

use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, HostingError>;

#[derive(Debug, thiserror::Error)]
pub enum HostingError {
    #[error("game server hosting is not configured on this bot")]
    Disabled,

    #[error("`{0}` is not a game I can host")]
    UnknownGame(String),

    #[error("`{0}` is not a plan I offer")]
    UnknownPlan(String),

    #[error("that server does not exist, or is not yours")]
    ServerNotFound,

    #[error(
        "you already have {current} of {allowed} servers. Cancel one, or \
         upgrade for a higher cap"
    )]
    ServerCap { current: i64, allowed: i64 },

    #[error(
        "the box is full: {0} servers are already running and there are no \
         free allocations left"
    )]
    NoCapacity(i64),

    #[error(
        "not enough memory left on the box for a {requested} MiB server \
         ({available} MiB free)"
    )]
    NoMemory { requested: i64, available: i64 },

    #[error("that server is still being set up; give it a moment")]
    NotReady,

    #[error("the panel is unreachable right now; please try again shortly")]
    PanelUnavailable,

    #[error("the panel rejected the request: {0}")]
    Panel(String),

    #[error("the panel never finished installing the server")]
    InstallTimeout,

    #[error(transparent)]
    Database(#[from] sqlx::Error),

    #[error(transparent)]
    Discord(#[from] serenity::Error),

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error("internal error: {0}")]
    Internal(String),
}

impl HostingError {
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    pub fn panel(msg: impl Into<String>) -> Self {
        Self::Panel(msg.into())
    }
}

impl Respond for HostingError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Disabled
            | Self::UnknownGame(_)
            | Self::UnknownPlan(_)
            | Self::ServerNotFound
            | Self::ServerCap { .. }
            | Self::NoCapacity(_)
            | Self::NoMemory { .. }
            | Self::NotReady
            | Self::PanelUnavailable
            | Self::InstallTimeout => Some(Cow::Owned(self.to_string())),

            Self::Panel(_)
            | Self::Database(_)
            | Self::Discord(_)
            | Self::Http(_)
            | Self::Internal(_) => None,
        }
    }
}

impl From<HostingError> for HandlerError {
    fn from(e: HostingError) -> Self {
        match e {
            // Database and Discord failures are the handler's own categories;
            // routing them through `Module` would lose that classification.
            HostingError::Database(e) => Self::Database(e),
            HostingError::Discord(e) => Self::Discord(e),

            other @ (HostingError::Disabled
            | HostingError::UnknownGame(_)
            | HostingError::UnknownPlan(_)
            | HostingError::ServerNotFound
            | HostingError::ServerCap { .. }
            | HostingError::NoCapacity(_)
            | HostingError::NoMemory { .. }
            | HostingError::NotReady
            | HostingError::PanelUnavailable
            | HostingError::Panel(_)
            | HostingError::InstallTimeout
            | HostingError::Http(_)
            | HostingError::Internal(_)) => Self::from_respond(other),
        }
    }
}
