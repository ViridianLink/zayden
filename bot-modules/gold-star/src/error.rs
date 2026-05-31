use std::borrow::Cow;
use std::fmt::Display;

use zayden_core::error::Respond;

pub type Result<T> = std::result::Result<T, GoldStarError>;

#[derive(Debug)]
pub enum GoldStarError {
    SelfStar,
    NoStars(i64),
    InvalidOptions,
    Serenity(serenity::Error),
    Sqlx(sqlx::Error),
}

impl Display for GoldStarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SelfStar => write!(f, "You can't give yourself a star."),
            Self::NoStars(timestamp) => write!(
                f,
                "You don't have any stars to give.\nNext free star <t:{timestamp}:R>"
            ),
            Self::InvalidOptions => write!(f, "Invalid command options."),
            Self::Serenity(e) => write!(f, "serenity: {e:?}"),
            Self::Sqlx(e) => write!(f, "sqlx: {e:?}"),
        }
    }
}

impl std::error::Error for GoldStarError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serenity(e) => Some(e),
            Self::Sqlx(e) => Some(e),
            Self::SelfStar | Self::NoStars(_) | Self::InvalidOptions => None,
        }
    }
}

impl Respond for GoldStarError {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::Serenity(_) | Self::Sqlx(_) => None,
            Self::SelfStar | Self::NoStars(_) | Self::InvalidOptions => {
                Some(Cow::Owned(self.to_string()))
            },
        }
    }
}

impl From<serenity::Error> for GoldStarError {
    fn from(value: serenity::Error) -> Self {
        Self::Serenity(value)
    }
}

impl From<sqlx::Error> for GoldStarError {
    fn from(value: sqlx::Error) -> Self {
        Self::Sqlx(value)
    }
}
