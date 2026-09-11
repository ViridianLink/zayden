use std::borrow::Cow;

use zayden_core::error::{HandlerError, Respond};

use crate::transport::ApiError;

pub type Result<T> = std::result::Result<T, JellyfinError>;

#[derive(Debug, thiserror::Error)]
pub enum JellyfinError {
    #[error(transparent)]
    Discord(#[from] serenity::Error),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Api(#[from] ApiError),

    #[error(
        "The Jellyfin integration is not configured on this bot. Ask an admin to \
         fill in the [jellyfin] section of config.toml and set JELLYFIN_API_KEY \
         and JELLYSEERR_API_KEY."
    )]
    NotConfigured,
    #[error("This command can only be used in a server.")]
    MissingGuildId,
    #[error("Unknown subcommand: {0}")]
    UnknownSubcommand(String),

    #[error(
        "You have not linked a Jellyfin account yet. Run `/jellyfin link` first."
    )]
    NotLinked,
    #[error("{0} has not linked a Jellyfin account.")]
    TargetNotLinked(String),
    #[error(
        "You are already linked to the Jellyfin account **{0}**. Run `/jellyfin unlink` first."
    )]
    AlreadyLinked(String),
    #[error(
        "That Jellyfin account is already linked to a different Discord user. \
         If that was you, unlink it from the other account first."
    )]
    AccountClaimed,
    #[error(
        "Quick Connect is turned off on the Jellyfin server, so accounts cannot \
         be linked right now. Ask an admin to enable it."
    )]
    QuickConnectDisabled,

    #[error("Nothing on the server or on TMDB matches **{0}**.")]
    NoSuchTitle(String),
    #[error("**{0}** is not on the server yet. Request it with `/watch request`.")]
    NotOnServer(String),
    #[error(
        "I could not work out where **{0}** lives on disk, so I cannot host a party for it."
    )]
    NoItemPath(String),
    #[error(
        "The library index is still empty. It refreshes on a schedule — try again in a few minutes."
    )]
    EmptyIndex,

    #[error("{0} keeps their watch streak private.")]
    StreakPrivate(String),
    #[error("Guest accounts are disabled in this server.")]
    GuestsDisabled,
    #[error(
        "That would put this server over its guest limit of {limit}. Wait for a \
         party to finish, or raise the limit from the dashboard."
    )]
    GuestLimit { limit: i32 },
    #[error("No party with id {0}.")]
    NoSuchParty(i64),
    #[error("Only the party host or a moderator can do that.")]
    NotPartyHost,
    #[error(
        "`{0}` is not a time I can read. Try something like `2026-09-14 20:00`."
    )]
    BadTime(String),
    #[error("A party has to start in the future.")]
    PartyInThePast,

    #[error("The AI backend is not configured, so this command is unavailable.")]
    AiUnavailable,
    #[error("The AI backend could not answer that: {0}")]
    Ai(String),
    #[error(
        "Content warnings need a DoesTheDogDie API key, which this bot does not have."
    )]
    WarningsUnavailable,
    #[error("Letterboxd has no public diary for **{0}**.")]
    NoLetterboxdFeed(String),
    #[error("Could not read that Letterboxd feed: {0}")]
    LetterboxdParse(String),
    #[error(
        "Letterboxd would not accept those details. Check the username and \
         password, then try again."
    )]
    LetterboxdAuth,

    #[error(
        "[jellyfin].seer_base_url ({url}) is not a usable Jellyseerr base URL: \
         {reason}"
    )]
    InvalidSeerBaseUrl { url: String, reason: String },

    #[error("internal error: {0}")]
    Internal(String),
}

impl Respond for JellyfinError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::NotConfigured
            | Self::MissingGuildId
            | Self::UnknownSubcommand(_)
            | Self::NotLinked
            | Self::TargetNotLinked(_)
            | Self::AlreadyLinked(_)
            | Self::AccountClaimed
            | Self::QuickConnectDisabled
            | Self::NoSuchTitle(_)
            | Self::NotOnServer(_)
            | Self::NoItemPath(_)
            | Self::EmptyIndex
            | Self::StreakPrivate(_)
            | Self::GuestsDisabled
            | Self::GuestLimit { .. }
            | Self::NoSuchParty(_)
            | Self::NotPartyHost
            | Self::BadTime(_)
            | Self::PartyInThePast
            | Self::AiUnavailable
            | Self::WarningsUnavailable
            | Self::NoLetterboxdFeed(_)
            | Self::LetterboxdAuth => Some(Cow::Owned(self.to_string())),

            // These can carry an upstream body, a URL or a key, so they go to
            // the log and the user sees nothing.
            Self::Discord(_)
            | Self::Database(_)
            | Self::Api(_)
            | Self::Ai(_)
            | Self::LetterboxdParse(_)
            | Self::InvalidSeerBaseUrl { .. }
            | Self::Internal(_) => None,
        }
    }
}

impl From<JellyfinError> for HandlerError {
    fn from(e: JellyfinError) -> Self {
        Self::from_respond(e)
    }
}

impl From<HandlerError> for JellyfinError {
    fn from(e: HandlerError) -> Self {
        match e {
            HandlerError::Discord(e) => Self::Discord(e),
            HandlerError::Database(e) => Self::Database(e),
            HandlerError::Module { source, .. } => {
                Self::Internal(source.to_string())
            },
        }
    }
}
