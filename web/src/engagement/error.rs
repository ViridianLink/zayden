use std::fmt::Display;

use topcoat::router::error::SeeOther;
use twilight_http::response::DeserializeBodyError;

use crate::auth::AuthError;
use crate::form::FieldError;
use crate::guild::GuildError;

/// Why an engagement call failed.
///
/// Each message is the exact text the dashboard shows: render it through
/// [`server_error_text`](crate::util::server_error_text).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EngagementError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Guild(#[from] GuildError),
    #[error("{0}")]
    Server(String),
    #[error("invalid {0}")]
    Invalid(&'static str),
    #[error("that emoji is already mapped on that message")]
    EmojiAlreadyMapped,
    #[error("that channel is already on the list")]
    ChannelAlreadyListed,
    #[error(
        "Discord allows at most {0} channels per command. Remove one before \
         adding another."
    )]
    ChannelListFull(usize),
    #[error("that channel is not on this server's list")]
    ChannelNotListed,
    #[error("that image is not in this server's list")]
    ImageNotFound,
    #[error(
        "On the {tier} plan the {label} cooldown can't go below {floor}s. \
         {upgrade}"
    )]
    CooldownBelowFloor {
        tier: &'static str,
        label: &'static str,
        floor: i32,
        upgrade: String,
    },
    #[error("unknown field `{0}`")]
    UnknownField(String),
    #[error("duplicate field `{0}`")]
    DuplicateField(String),
    #[error("missing field `{0}`")]
    MissingField(&'static str),
    #[error("the form's guild is not the page's guild")]
    GuildMismatch,
}

impl EngagementError {
    /// Whether the caller was refused, as opposed to the server failing to
    /// serve them.
    #[must_use]
    pub const fn is_denied(&self) -> bool {
        match self {
            Self::Auth(auth) => auth.is_denied(),
            Self::Guild(guild) => guild.is_denied(),
            Self::Server(_)
            | Self::Invalid(_)
            | Self::EmojiAlreadyMapped
            | Self::ChannelAlreadyListed
            | Self::ChannelListFull(_)
            | Self::ChannelNotListed
            | Self::ImageNotFound
            | Self::CooldownBelowFloor { .. }
            | Self::UnknownField(_)
            | Self::DuplicateField(_)
            | Self::MissingField(_)
            | Self::GuildMismatch => false,
        }
    }

    /// Whether the submitted form did not have the shape the save expects: an
    /// unknown, repeated or missing field, or a `guild` field that is not the
    /// page's guild. Pages answer these with 422.
    #[must_use]
    pub const fn is_invalid_form(&self) -> bool {
        matches!(
            self,
            Self::UnknownField(_)
                | Self::DuplicateField(_)
                | Self::MissingField(_)
                | Self::GuildMismatch
        )
    }

    /// A 303 to the login page for an unauthenticated caller, so `?` sends a
    /// signed-out visitor there; any other error comes back for the page to
    /// render inline.
    pub fn redirect_unauthenticated(self) -> Result<Self, SeeOther> {
        if let Self::Auth(auth) = self {
            return auth.redirect_unauthenticated().map(Self::Auth);
        }

        if let Self::Guild(guild) = self {
            return guild.redirect_unauthenticated().map(Self::Guild);
        }

        Ok(self)
    }
}

pub(crate) fn server_err(e: impl Display) -> EngagementError {
    EngagementError::Server(e.to_string())
}

impl From<FieldError> for EngagementError {
    fn from(e: FieldError) -> Self {
        match e {
            FieldError::Unknown(name) => Self::UnknownField(name),
            FieldError::Duplicate(name) => Self::DuplicateField(name),
            FieldError::Missing(name) => Self::MissingField(name),
        }
    }
}

impl From<sqlx::Error> for EngagementError {
    fn from(e: sqlx::Error) -> Self {
        server_err(e)
    }
}

impl From<twilight_http::Error> for EngagementError {
    fn from(e: twilight_http::Error) -> Self {
        server_err(e)
    }
}

impl From<DeserializeBodyError> for EngagementError {
    fn from(e: DeserializeBodyError) -> Self {
        server_err(e)
    }
}

impl From<greetings::GreetingsError> for EngagementError {
    fn from(e: greetings::GreetingsError) -> Self {
        server_err(e)
    }
}

impl From<reaction_roles::ReactionRoleError> for EngagementError {
    fn from(e: reaction_roles::ReactionRoleError) -> Self {
        server_err(e)
    }
}
