use serenity::all::{ChannelId, Mentionable};

pub(crate) type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum PermissionError {
    NotOwner,
    NotTrusted,
}

#[derive(Debug)]
pub enum Error {
    MissingGuildId,
    MemberNotInVoiceChannel,
    OwnerInChannel,
    InvalidPassword,
    UserIsOwner,
    MaxChannels,
    MissingPermissions(PermissionError),
    ChannelNotFound(ChannelId),

    Serenity(serenity::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::MissingGuildId => zayden_core::Error::MissingGuildId.fmt(f),
            Error::MemberNotInVoiceChannel => {
                write!(
                    f,
                    "You must be in a voice channel or use the `channel` option to specify a channel to use this command."
                )
            }
            Error::OwnerInChannel => {
                write!(
                    f,
                    "Cannot use this command while the channel owner is present."
                )
            }
            Error::InvalidPassword => write!(f, "Invalid channel password."),
            Error::UserIsOwner => write!(f, "You are already the owner of this channel."),
            Error::MaxChannels => write!(
                f,
                "You have reached the maximum number of persistent channels."
            ),
            Error::MissingPermissions(PermissionError::NotOwner) => {
                write!(f, "Only the channel owner can use this command.")
            }
            Error::MissingPermissions(PermissionError::NotTrusted) => {
                write!(f, "You must be trusted to use this command.")
            }
            Error::ChannelNotFound(id) => write!(
                f,
                "Channel not found: {}\nTry using `/voice claim` to claim the channel.",
                id.mention()
            ),
            Self::Serenity(_) => unimplemented!("Serenity errors should be handled in application"),
        }
    }
}

impl std::error::Error for Error {}

impl From<serenity::Error> for Error {
    fn from(value: serenity::Error) -> Self {
        Self::Serenity(value)
    }
}
