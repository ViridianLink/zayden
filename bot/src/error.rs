use zayden_core::{Error as ZaydenError, error::Respond};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    NotInteractionAuthor,
    NegativeHours,

    EndgameAnalysis(endgame_analysis::Error),
    Gambling(gambling::Error),
    Lfg(lfg::Error),
    ReactionRole(reaction_roles::Error),
    Ticket(ticket::Error),
    Suggestions(suggestions::Error),
    TempVoice(temp_voice::Error),

    Ai(ai::Error),

    ZaydenCore(ZaydenError),

    Config(zayden_app::Error),
    Jiff(jiff::Error),
    EnvVar(std::env::VarError),
    Other(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::NotInteractionAuthor => {
                write!(f, "You are not the author of this interaction.")
            }
            Error::NegativeHours => write!(f, "Hours must be a positive number."),

            Error::ZaydenCore(e) => e.fmt(f),

            Error::EndgameAnalysis(e) => e.fmt(f),
            Error::Gambling(e) => e.fmt(f),
            Error::Lfg(e) => e.fmt(f),
            Error::ReactionRole(e) => e.fmt(f),
            Error::Ticket(e) => e.fmt(f),
            Error::Suggestions(e) => e.fmt(f),
            Error::TempVoice(e) => e.fmt(f),
            Error::Ai(e) => e.fmt(f),
            Error::Config(e) => e.fmt(f),
            Error::Jiff(e) => e.fmt(f),
            Error::EnvVar(e) => e.fmt(f),
            Error::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::EndgameAnalysis(e) => Some(e),
            Self::Gambling(e) => Some(e),
            Self::Lfg(e) => Some(e),
            Self::ReactionRole(e) => Some(e),
            Self::Ticket(e) => Some(e),
            Self::Suggestions(e) => Some(e),
            Self::TempVoice(e) => Some(e),
            Self::Ai(e) => Some(e),
            Self::Config(e) => Some(e),
            Self::ZaydenCore(e) => Some(e),
            Self::Jiff(e) => Some(e),
            Self::EnvVar(e) => Some(e),
            _ => None,
        }
    }
}

impl Respond for Error {
    fn user_message(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::NotInteractionAuthor | Self::NegativeHours => {
                Some(std::borrow::Cow::Owned(self.to_string()))
            }

            Self::EndgameAnalysis(e) => e.user_message(),
            Self::Gambling(e) => e.user_message(),
            Self::Lfg(e) => e.user_message(),
            Self::ReactionRole(e) => e.user_message(),
            Self::Ticket(e) => e.user_message(),
            Self::Suggestions(e) => e.user_message(),
            Self::TempVoice(e) => e.user_message(),

            Self::ZaydenCore(e) => e.user_message(),

            Self::Ai(_) | Self::Config(_) | Self::Jiff(_) | Self::EnvVar(_) | Self::Other(_) => {
                None
            }
        }
    }
}

impl From<endgame_analysis::Error> for Error {
    fn from(e: endgame_analysis::Error) -> Self {
        Error::EndgameAnalysis(e)
    }
}

impl From<gambling::Error> for Error {
    fn from(value: gambling::Error) -> Self {
        match value {
            gambling::Error::Serenity(e) => Self::ZaydenCore(ZaydenError::Serenity(e)),
            gambling::Error::Sqlx(e) => Self::ZaydenCore(ZaydenError::Sqlx(e)),
            value => Self::Gambling(value),
        }
    }
}

impl From<lfg::Error> for Error {
    fn from(value: lfg::Error) -> Self {
        match value {
            lfg::Error::Serenity(e) => Self::ZaydenCore(ZaydenError::Serenity(e)),
            value => Self::Lfg(value),
        }
    }
}

impl From<reaction_roles::Error> for Error {
    fn from(e: reaction_roles::Error) -> Self {
        Error::ReactionRole(e)
    }
}

impl From<temp_voice::Error> for Error {
    fn from(value: temp_voice::Error) -> Self {
        match value {
            temp_voice::Error::Serenity(e) => Self::ZaydenCore(ZaydenError::Serenity(e)),
            value => Error::TempVoice(value),
        }
    }
}

impl From<ticket::Error> for Error {
    fn from(value: ticket::Error) -> Self {
        match value {
            ticket::Error::ZaydenCore(e) => Self::ZaydenCore(e),
            value => Self::Ticket(value),
        }
    }
}

impl From<suggestions::Error> for Error {
    fn from(value: suggestions::Error) -> Self {
        match value {
            suggestions::Error::Zayden(e) => Self::ZaydenCore(e),
            value => Self::Suggestions(value),
        }
    }
}

impl From<serenity::Error> for Error {
    fn from(value: serenity::Error) -> Self {
        Self::ZaydenCore(ZaydenError::Serenity(value))
    }
}

impl From<sqlx::Error> for Error {
    fn from(value: sqlx::Error) -> Self {
        Self::ZaydenCore(ZaydenError::Sqlx(value))
    }
}

impl From<jiff::Error> for Error {
    fn from(e: jiff::Error) -> Self {
        Self::Jiff(e)
    }
}

impl From<ai::Error> for Error {
    fn from(e: ai::Error) -> Self {
        Self::Ai(e)
    }
}

impl From<std::env::VarError> for Error {
    fn from(e: std::env::VarError) -> Self {
        Self::EnvVar(e)
    }
}

impl From<zayden_app::Error> for Error {
    fn from(e: zayden_app::Error) -> Self {
        Self::Config(e)
    }
}
