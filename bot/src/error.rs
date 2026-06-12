use zayden_core::CoreError;
use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, BotError>;

#[derive(Debug)]
pub enum BotError {
    NotInteractionAuthor,
    NegativeHours,

    EndgameAnalysis(endgame_analysis::EndgameAnalysisError),
    Lfg(lfg::LfgError),
    ReactionRole(reaction_roles::ReactionRoleError),
    Suggestions(suggestions::SuggestionsError),
    Ticket(ticket::TicketError),
    TempVoice(temp_voice::TempVoiceError),

    Ai(ai::Error),

    ZaydenCore(CoreError),

    Config(zayden_app::AppError),
    Jiff(jiff::Error),
    EnvVar(std::env::VarError),
    Other(String),
}

impl std::fmt::Display for BotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotInteractionAuthor => {
                write!(f, "You are not the author of this interaction.")
            },
            Self::NegativeHours => {
                write!(f, "Hours must be a positive number.")
            },

            Self::ZaydenCore(e) => e.fmt(f),

            Self::EndgameAnalysis(e) => e.fmt(f),
            Self::Lfg(e) => e.fmt(f),
            Self::ReactionRole(e) => e.fmt(f),
            Self::Suggestions(e) => e.fmt(f),
            Self::Ticket(e) => e.fmt(f),
            Self::TempVoice(e) => e.fmt(f),
            Self::Ai(e) => e.fmt(f),
            Self::Config(e) => e.fmt(f),
            Self::Jiff(e) => e.fmt(f),
            Self::EnvVar(e) => e.fmt(f),
            Self::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for BotError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::EndgameAnalysis(e) => Some(e),
            Self::Lfg(e) => Some(e),
            Self::ReactionRole(e) => Some(e),
            Self::Suggestions(e) => Some(e),
            Self::Ticket(e) => Some(e),
            Self::TempVoice(e) => Some(e),
            Self::Ai(e) => Some(e),
            Self::Config(e) => Some(e),
            Self::ZaydenCore(e) => Some(e),
            Self::Jiff(e) => Some(e),
            Self::EnvVar(e) => Some(e),
            Self::NotInteractionAuthor | Self::NegativeHours | Self::Other(_) => {
                None
            },
        }
    }
}

impl Respond for BotError {
    fn user_message(&self) -> Option<std::borrow::Cow<'_, str>> {
        match self {
            Self::NotInteractionAuthor | Self::NegativeHours => {
                Some(std::borrow::Cow::Owned(self.to_string()))
            },

            Self::EndgameAnalysis(e) => e.user_message(),
            Self::Lfg(e) => e.user_message(),
            Self::ReactionRole(e) => e.user_message(),
            Self::Suggestions(e) => e.user_message(),
            Self::Ticket(e) => e.user_message(),
            Self::TempVoice(e) => e.user_message(),

            Self::ZaydenCore(e) => e.user_message(),

            Self::Ai(_)
            | Self::Config(_)
            | Self::Jiff(_)
            | Self::EnvVar(_)
            | Self::Other(_) => None,
        }
    }
}

impl From<endgame_analysis::EndgameAnalysisError> for BotError {
    fn from(e: endgame_analysis::EndgameAnalysisError) -> Self {
        Self::EndgameAnalysis(e)
    }
}

impl From<lfg::LfgError> for BotError {
    fn from(e: lfg::LfgError) -> Self {
        Self::Lfg(e)
    }
}

impl From<reaction_roles::ReactionRoleError> for BotError {
    fn from(e: reaction_roles::ReactionRoleError) -> Self {
        Self::ReactionRole(e)
    }
}

impl From<suggestions::SuggestionsError> for BotError {
    fn from(e: suggestions::SuggestionsError) -> Self {
        Self::Suggestions(e)
    }
}

impl From<temp_voice::TempVoiceError> for BotError {
    fn from(e: temp_voice::TempVoiceError) -> Self {
        Self::TempVoice(e)
    }
}

impl From<ticket::TicketError> for BotError {
    fn from(e: ticket::TicketError) -> Self {
        Self::Ticket(e)
    }
}

impl From<serenity::Error> for BotError {
    fn from(value: serenity::Error) -> Self {
        Self::ZaydenCore(CoreError::Serenity(value))
    }
}

impl From<sqlx::Error> for BotError {
    fn from(value: sqlx::Error) -> Self {
        Self::ZaydenCore(CoreError::Sqlx(value))
    }
}

impl From<jiff::Error> for BotError {
    fn from(e: jiff::Error) -> Self {
        Self::Jiff(e)
    }
}

impl From<ai::Error> for BotError {
    fn from(e: ai::Error) -> Self {
        Self::Ai(e)
    }
}

impl From<bungie_api::BungieApiError> for BotError {
    fn from(e: bungie_api::BungieApiError) -> Self {
        Self::Other(e.to_string())
    }
}

impl From<std::io::Error> for BotError {
    fn from(e: std::io::Error) -> Self {
        Self::Other(e.to_string())
    }
}

impl From<std::env::VarError> for BotError {
    fn from(e: std::env::VarError) -> Self {
        Self::EnvVar(e)
    }
}

impl From<zayden_app::AppError> for BotError {
    fn from(e: zayden_app::AppError) -> Self {
        Self::Config(e)
    }
}

impl From<HandlerError> for BotError {
    fn from(e: HandlerError) -> Self {
        match e {
            HandlerError::Database(e) => Self::ZaydenCore(CoreError::Sqlx(e)),
            HandlerError::Discord(e) => Self::ZaydenCore(CoreError::Serenity(e)),
            HandlerError::Module { source, .. } => {
                Self::ZaydenCore(CoreError::Other(source.to_string()))
            },
        }
    }
}
