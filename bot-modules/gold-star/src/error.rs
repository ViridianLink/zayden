use std::fmt::Display;

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
