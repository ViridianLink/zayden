use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Tier {
    Free,
    Pro,
    Enterprise,
}

impl Tier {
    pub fn as_str(self) -> &'static str {
        match self {
            Tier::Free => "free",
            Tier::Pro => "pro",
            Tier::Enterprise => "enterprise",
        }
    }
}

impl std::str::FromStr for Tier {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "free" => Ok(Tier::Free),
            "pro" => Ok(Tier::Pro),
            "enterprise" => Ok(Tier::Enterprise),
            _ => Err(()),
        }
    }
}

/// Identifies the principal (user, guild, or both) for an entitlement check.
///
/// `scope_id` maps to `user_id` for `User`, `guild_id` for `Guild`, and
/// `(user_id, guild_id)` for `UserInGuild`. The integer widths match Serenity's
/// `UserId`/`GuildId` newtypes without creating a Serenity dependency in this crate.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EntitlementScope {
    User(u64),
    Guild(u64),
    UserInGuild(u64, u64),
}

impl EntitlementScope {
    pub(crate) fn scope_type(&self) -> &'static str {
        match self {
            EntitlementScope::User(_) => "user",
            EntitlementScope::Guild(_) => "guild",
            EntitlementScope::UserInGuild(_, _) => "user_in_guild",
        }
    }

    pub(crate) fn scope_id(&self) -> i64 {
        match self {
            EntitlementScope::User(id) | EntitlementScope::Guild(id) => *id as i64,
            EntitlementScope::UserInGuild(user_id, _) => *user_id as i64,
        }
    }

    pub(crate) fn scope_secondary_id(&self) -> i64 {
        match self {
            EntitlementScope::UserInGuild(_, guild_id) => *guild_id as i64,
            _ => 0,
        }
    }
}
