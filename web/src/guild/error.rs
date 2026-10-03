use std::fmt::Display;

use topcoat::router::error::SeeOther;
use twilight_http::response::DeserializeBodyError;

use crate::auth::AuthError;

/// Why a guild data call failed.
///
/// Each message is the exact text the dashboard shows: render it through
/// [`server_error_text`](crate::util::server_error_text).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GuildError {
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error("{0}")]
    Server(String),
    #[error("unknown field `{0}`")]
    UnknownField(String),
    #[error("duplicate field `{0}`")]
    DuplicateField(String),
    #[error("missing field `{0}`")]
    MissingField(&'static str),
    #[error("invalid value for `{0}`")]
    InvalidField(&'static str),
    #[error("invalid wiki URL")]
    InvalidWikiUrl,
    #[error("the wiki URL must start with http:// or https://")]
    WikiUrlScheme,
    #[error("invalid role")]
    InvalidRole,
    #[error("that role is already a support role")]
    DuplicateSupportRole,
    #[error("invalid user id")]
    InvalidUserId,
    #[error("invalid link: {0}")]
    InvalidLink(String),
    #[error("link must be an http:// or https:// address")]
    LinkScheme,
    #[error("link must not embed credentials")]
    LinkCredentials,
    #[error("link is too long")]
    LinkTooLong,
    #[error("select a category first")]
    NoCategory,
    #[error("invalid channel id")]
    InvalidChannelId,
    #[error("invalid email")]
    InvalidEmail,
    #[error("This Ko-fi email is already linked to an account.")]
    KofiEmailTaken,
    #[error("invalid application id")]
    InvalidApplicationId,
    #[error("unknown module")]
    UnknownModule,
    #[error("module {0} has no settings switch")]
    NoSettingsSwitch(String),
    #[error("{0} is switched on from its own settings page, not from this toggle.")]
    DerivedModule(&'static str),
    #[error("/{0} isn't registered for this server yet")]
    CommandNotRegistered(String),
    #[error(
        "Discord only lets a member with Manage Server change command \
         permissions, so /{0} can't be changed through operator access."
    )]
    OperatorCommandPermissions(String),
    #[error("Discord rejected the permission update for /{name}: {reason}")]
    PermissionUpdateRejected { name: String, reason: String },
}

impl GuildError {
    /// Whether the caller was refused, as opposed to the server failing to
    /// serve them.
    #[must_use]
    pub const fn is_denied(&self) -> bool {
        matches!(self, Self::Auth(auth) if auth.is_denied())
    }

    /// Whether the submitted form did not have the shape the save expects:
    /// an unknown, repeated or missing field. Pages answer these with 422.
    #[must_use]
    pub const fn is_invalid_form(&self) -> bool {
        matches!(
            self,
            Self::UnknownField(_)
                | Self::DuplicateField(_)
                | Self::MissingField(_)
                | Self::InvalidField(_)
        )
    }

    /// A 303 to the login page for an unauthenticated caller, so `?` sends a
    /// signed-out visitor there; any other error comes back for the page to
    /// render inline.
    pub fn redirect_unauthenticated(self) -> Result<Self, SeeOther> {
        let Self::Auth(auth) = self else {
            return Ok(self);
        };
        auth.redirect_unauthenticated().map(Self::Auth)
    }
}

pub(crate) fn server_err(e: impl Display) -> GuildError {
    GuildError::Server(e.to_string())
}

impl From<sqlx::Error> for GuildError {
    fn from(e: sqlx::Error) -> Self {
        server_err(e)
    }
}

impl From<twilight_http::Error> for GuildError {
    fn from(e: twilight_http::Error) -> Self {
        server_err(e)
    }
}

impl From<DeserializeBodyError> for GuildError {
    fn from(e: DeserializeBodyError) -> Self {
        server_err(e)
    }
}
