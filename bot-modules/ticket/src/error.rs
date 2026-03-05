use zayden_core::Error as ZaydenError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    MissingGuildId,
    NotInSupportChannel,

    ZaydenCore(ZaydenError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::MissingGuildId => ZaydenError::MissingGuildId.fmt(f),
            Error::NotInSupportChannel => {
                write!(f, "This command only works in the support channel.")
            }
            Self::ZaydenCore(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl From<serenity::Error> for Error {
    fn from(value: serenity::Error) -> Self {
        Self::ZaydenCore(ZaydenError::Serenity(value))
    }
}
