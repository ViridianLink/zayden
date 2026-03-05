use std::fmt::Display;

use serenity::all::{Mentionable, UserId};

use crate::relationships::Relationships;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    // region common
    Zayden,
    Bot,
    InvalidUserId,
    AlreadyRelated {
        target: UserId,
        relationship: Relationships,
    },
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

    //region children
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
            Self::AlreadyRelated {
                target,
                relationship,
            } => {
                write!(
                    f,
                    "You guys are already related! {} is your {relationship}.",
                    target.mention()
                )
            }
            Self::MaxPartners => {
                write!(f, "You're already at your partner limit! Use `/divorce` to break up with someone.")
            }
            Self::UnauthorisedUser => write!(f, "You can't respond to this interaction."),
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
            }
            e => unimplemented!("Unhandled Error Display: {e:?}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<serenity::Error> for Error {
    fn from(e: serenity::Error) -> Self {
        Error::Serenity(e)
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Error::Sqlx(e)
    }
}

impl From<std::env::VarError> for Error {
    fn from(e: std::env::VarError) -> Self {
        Error::EnvVar(e)
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
        Error::SerenityTimestamp(e)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(e: std::num::ParseIntError) -> Self {
        Error::ParseIntError(e)
    }
}

impl From<serenity::all::ReactionConversionError> for Error {
    fn from(e: serenity::all::ReactionConversionError) -> Self {
        Error::ReactionConversionError(e)
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
