use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Tier {
    Free,
    Pro,
    Enterprise,
}

impl Tier {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Pro => "pro",
            Self::Enterprise => "enterprise",
        }
    }
}

impl std::str::FromStr for Tier {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "free" => Ok(Self::Free),
            "pro" => Ok(Self::Pro),
            "enterprise" => Ok(Self::Enterprise),
            _ => Err(()),
        }
    }
}

/// Identifies the principal (user, guild, or both) for an entitlement check.
///
/// `scope_id` maps to `user_id` for `User`, `guild_id` for `Guild`, and
/// `(user_id, guild_id)` for `UserInGuild`. The integer widths match Serenity's
/// `UserId`/`GuildId` newtypes without creating a Serenity dependency in this
/// crate.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntitlementScope {
    User(u64),
    Guild(u64),
    UserInGuild(u64, u64),
}

impl EntitlementScope {
    #[must_use]
    pub(crate) const fn scope_type(&self) -> &'static str {
        match self {
            Self::User(_) => "user",
            Self::Guild(_) => "guild",
            Self::UserInGuild(..) => "user_in_guild",
        }
    }

    #[must_use]
    pub(crate) fn scope_id(&self) -> i64 {
        match self {
            Self::User(id) | Self::Guild(id) => {
                i64::try_from(*id).unwrap_or(i64::MAX)
            },
            Self::UserInGuild(user_id, _) => {
                i64::try_from(*user_id).unwrap_or(i64::MAX)
            },
        }
    }

    #[must_use]
    pub(crate) fn scope_secondary_id(&self) -> i64 {
        match self {
            Self::UserInGuild(_, guild_id) => {
                i64::try_from(*guild_id).unwrap_or(i64::MAX)
            },
            Self::User(_) | Self::Guild(_) => 0,
        }
    }
}
