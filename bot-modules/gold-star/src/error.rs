use std::borrow::Cow;
use std::fmt::Display;

use zayden_core::error::Respond;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    SelfStar,
    NoStars(i64),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelfStar => write!(f, "You can't give yourself a star."),
            Self::NoStars(timestamp) => write!(
                f,
                "You don't have any stars to give.\nNext free star <t:{timestamp}:R>"
            ),
        }
    }
}

impl std::error::Error for Error {}

impl Respond for Error {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        Some(Cow::Owned(self.to_string()))
    }
}
