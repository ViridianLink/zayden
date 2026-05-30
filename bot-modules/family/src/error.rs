use std::borrow::Cow;
use std::fmt::Display;

use serenity::all::{Mentionable, UserId};
use zayden_core::error::Respond;

use crate::relationships::Relationships;

pub type Result<T> = std::result::Result<T, Error>;

#[expect(clippy::error_impl_error, reason = "conventional error type naming")]
#[derive(Debug)]
pub enum Error {
    // region common
    Zayden,
    Bot,
    InvalidUserId,
    AlreadyRelated { target: UserId, relationship: Relationships },
    UnauthorisedUser,
    NoMentionedUser,
    NoInteraction,
    SameUser(UserId),
    NoData(UserId),
    // endregion

    // region adopt
    UserSelfAdopt,
    AlreadyAdopted(UserId),
    AdoptCancelled,
    // endregion

    // region block
    UserSelfBlock,
    // endregion

    // region children
    SelfNoChildren,
    NoChildren(UserId),
    // endregion

    // region marry
    UserSelfMarry,
    MaxPartners,
    MarryCancelled,
    // endregion

    // region parents
    SelfNoParents,
    NoParents(UserId),
    // endregion

    // region partners
    SelfNoPartners,
    NoPartners(UserId),
    // endregion

    // region siblings
    SelfNoSiblings,
    NoSiblings(UserId),
    // endregion

    // region external
    Serenity(serenity::Error),
    SerenityTimestamp(serenity::model::timestamp::InvalidTimestamp),
    Sqlx(sqlx::Error),
    EnvVar(std::env::VarError),
    // Reqwest(reqwest::Error),
    // Cron(cron::error::Error),
    ParseIntError(std::num::ParseIntError),
    ReactionConversionError(serenity::all::ReactionConversionError),
    // JoinError(tokio::task::JoinError),
    // CharmingError(charming::EchartsError),
    // endregion
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UserSelfMarry => write!(f, "You can't marry yourself!"),
            Self::Bot => write!(f, "Can robots even love?"),
            Self::Zayden => write!(f, "Please... I can do better than you."),
            Self::AlreadyRelated { target, relationship } => {
                write!(
                    f,
                    "You guys are already related! {} is your {relationship}.",
                    target.mention()
                )
            },
            Self::MaxPartners => {
                write!(
                    f,
                    "You're already at your partner limit! Use `/divorce` to break up with someone."
                )
            },
            Self::UnauthorisedUser => {
                write!(f, "You can't respond to this interaction.")
            },
            Self::SameUser(user_id) => write!(
                f,
                "Would you look at that... {0} is very closely related to {0}",
                user_id.mention()
            ),
            Self::UserSelfAdopt => write!(f, "You can't adopt yourself!"),
            Self::AlreadyAdopted(user_id) => {
                write!(
                    f,
                    "It looks like {} already has a parent.",
                    user_id.mention()
                )
            },
            Self::InvalidUserId
            | Self::NoMentionedUser
            | Self::NoInteraction
            | Self::NoData(_)
            | Self::AdoptCancelled
            | Self::UserSelfBlock
            | Self::SelfNoChildren
            | Self::NoChildren(_)
            | Self::MarryCancelled
            | Self::SelfNoParents
            | Self::NoParents(_)
            | Self::SelfNoPartners
            | Self::NoPartners(_)
            | Self::SelfNoSiblings
            | Self::NoSiblings(_)
            | Self::SerenityTimestamp(_)
            | Self::Sqlx(_)
            | Self::EnvVar(_)
            | Self::ParseIntError(_)
            | Self::ReactionConversionError(_)
            | Self::Serenity(_) => write!(f, "{self:?}"),
        }
    }
}

impl Respond for Error {
    fn user_message(&self) -> Option<Cow<'_, str>> {
        match self {
            Self::UserSelfMarry
            | Self::Bot
            | Self::Zayden
            | Self::AlreadyRelated { .. }
            | Self::MaxPartners
            | Self::UnauthorisedUser
            | Self::SameUser(_)
            | Self::UserSelfAdopt
            | Self::AlreadyAdopted(_) => Some(Cow::Owned(self.to_string())),
            Self::InvalidUserId
            | Self::NoMentionedUser
            | Self::NoInteraction
            | Self::NoData(_)
            | Self::AdoptCancelled
            | Self::UserSelfBlock
            | Self::SelfNoChildren
            | Self::NoChildren(_)
            | Self::MarryCancelled
            | Self::SelfNoParents
            | Self::NoParents(_)
            | Self::SelfNoPartners
            | Self::NoPartners(_)
            | Self::SelfNoSiblings
            | Self::NoSiblings(_)
            | Self::SerenityTimestamp(_)
            | Self::Sqlx(_)
            | Self::EnvVar(_)
            | Self::ParseIntError(_)
            | Self::ReactionConversionError(_)
            | Self::Serenity(_) => None,
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Serenity(e) => Some(e),
            Self::SerenityTimestamp(e) => Some(e),
            Self::Sqlx(e) => Some(e),
            Self::EnvVar(e) => Some(e),
            Self::ParseIntError(e) => Some(e),
            Self::ReactionConversionError(e) => Some(e),
            Self::UserSelfMarry
            | Self::MaxPartners
            | Self::Zayden
            | Self::Bot
            | Self::InvalidUserId
            | Self::AlreadyRelated { .. }
            | Self::UnauthorisedUser
            | Self::NoMentionedUser
            | Self::NoInteraction
            | Self::SameUser(_)
            | Self::NoData(_)
            | Self::UserSelfAdopt
            | Self::AlreadyAdopted(_)
            | Self::AdoptCancelled
            | Self::UserSelfBlock
            | Self::SelfNoChildren
            | Self::NoChildren(_)
            | Self::MarryCancelled
            | Self::SelfNoParents
            | Self::NoParents(_)
            | Self::SelfNoPartners
            | Self::NoPartners(_)
            | Self::SelfNoSiblings
            | Self::NoSiblings(_) => None,
        }
    }
}

impl From<serenity::Error> for Error {
    fn from(e: serenity::Error) -> Self {
        Self::Serenity(e)
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::Sqlx(e)
    }
}

impl From<std::env::VarError> for Error {
    fn from(e: std::env::VarError) -> Self {
        Self::EnvVar(e)
    }
}

// impl From<reqwest::Error> for Error {
//     fn from(e: reqwest::Error) -> Self {
//         Error::Reqwest(e)
//     }
// }

// impl From<cron::error::Error> for Error {
//     fn from(e: cron::error::Error) -> Self {
//         Error::Cron(e)
//     }
// }

impl From<serenity::model::timestamp::InvalidTimestamp> for Error {
    fn from(e: serenity::model::timestamp::InvalidTimestamp) -> Self {
        Self::SerenityTimestamp(e)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(e: std::num::ParseIntError) -> Self {
        Self::ParseIntError(e)
    }
}

impl From<serenity::all::ReactionConversionError> for Error {
    fn from(e: serenity::all::ReactionConversionError) -> Self {
        Self::ReactionConversionError(e)
    }
}

// impl From<tokio::task::JoinError> for Error {
//     fn from(e: tokio::task::JoinError) -> Self {
//         Error::JoinError(e)
//     }
// }

// impl From<charming::EchartsError> for Error {
//     fn from(e: charming::EchartsError) -> Self {
//         Error::CharmingError(e)
//     }
// }
