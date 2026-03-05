use zayden_core::Error as ZaydenError;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    MissingGuildId,
    MissingSuggesionChannel,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::MissingGuildId => ZaydenError::MissingGuildId.fmt(f),
            Error::MissingSuggesionChannel => {
                write!(f, "Please specify a channel to fetch suggestions from.")
            }
        }
    }
}

impl std::error::Error for Error {}
