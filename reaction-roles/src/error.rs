use serenity::all::ReactionConversionError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    MissingGuildId,
    InvalidMessageId(String),
    ReactionConversionError(ReactionConversionError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::MissingGuildId => zayden_core::Error::MissingGuildId.fmt(f),
            Self::InvalidMessageId(id) => write!(f, "Invalid message ID: {id}"),
            Self::ReactionConversionError(_) => write!(f, "Failed to convert emoji to reaction"),
        }
    }
}

impl std::error::Error for Error {}

impl From<ReactionConversionError> for Error {
    fn from(err: ReactionConversionError) -> Self {
        Self::ReactionConversionError(err)
    }
}
