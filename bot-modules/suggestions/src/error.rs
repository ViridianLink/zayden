use std::borrow::Cow;

use zayden_core::error::Respond;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    MissingSuggesionChannel,
    Zayden(zayden_core::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::MissingSuggesionChannel => {
                write!(f, "Please specify a channel to fetch suggestions from.")
            }
            Self::Zayden(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

impl Respond for Error {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Error::MissingSuggesionChannel => Some(Cow::Owned(self.to_string())),
            Self::Zayden(e) => e.user_message(),
        }
    }
}

impl From<zayden_core::Error> for Error {
    fn from(value: zayden_core::Error) -> Self {
        Self::Zayden(value)
    }
}
