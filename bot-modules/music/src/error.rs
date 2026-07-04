use std::borrow::Cow;

use zayden_core::error::{HandlerError, Respond};

pub type Result<T> = std::result::Result<T, MusicError>;

#[derive(Debug, thiserror::Error)]
pub enum MusicError {
    #[error("You need to be in a voice channel to use this command.")]
    UserNotInVoice,
    #[error("I'm not currently connected to a voice channel in this server.")]
    NotConnected,
    #[error("Nothing is currently playing.")]
    NothingPlaying,
    #[error("The queue is empty.")]
    QueueEmpty,
    #[error("Position {0} is out of range for the current queue.")]
    QueuePositionOutOfRange(usize),
    #[error("You need the DJ role or Manage Server permission to do that.")]
    NotPrivileged,
    #[error("That playlist has too many tracks (max {max}); the first {max} were queued.")]
    PlaylistTruncated { max: usize },
    #[error("Couldn't find any results for that query.")]
    NoResults,
    #[error("That doesn't look like a supported YouTube or Spotify link.")]
    UnsupportedSource,
    #[error("Seeking isn't supported on live streams.")]
    SeekOnLiveStream,
    #[error("That seek position is outside the track's duration.")]
    SeekOutOfRange,
    #[error("Volume must be between 0 and 100.")]
    VolumeOutOfRange,
    #[error("This feature requires a premium subscription.")]
    PremiumRequired,

    #[error("failed to resolve track: {0}")]
    Resolve(String),
    #[error("songbird error: {0}")]
    Songbird(String),

    #[error("discord error: {0}")]
    Serenity(#[from] serenity::Error),
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),
}

impl Respond for MusicError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::UserNotInVoice
            | Self::NotConnected
            | Self::NothingPlaying
            | Self::QueueEmpty
            | Self::QueuePositionOutOfRange(_)
            | Self::NotPrivileged
            | Self::PlaylistTruncated { .. }
            | Self::NoResults
            | Self::UnsupportedSource
            | Self::SeekOnLiveStream
            | Self::SeekOutOfRange
            | Self::VolumeOutOfRange
            | Self::PremiumRequired => Some(Cow::Owned(self.to_string())),
            Self::Resolve(_) | Self::Songbird(_) | Self::Serenity(_) | Self::Sqlx(_) => {
                None
            },
        }
    }
}

impl From<MusicError> for HandlerError {
    fn from(e: MusicError) -> Self {
        Self::from_respond(e)
    }
}
