use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct ParseError(String);

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Error for ParseError {}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum Tier {
    Free,
    Pro,
    Ultra,
}

impl Tier {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Pro => "pro",
            Self::Ultra => "ultra",
        }
    }
}

impl std::str::FromStr for Tier {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "free" => Ok(Self::Free),
            "pro" => Ok(Self::Pro),
            "ultra" => Ok(Self::Ultra),
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

    /// Encodes the scope as a `pg_notify` payload string:
    /// `"scope_type:scope_id:scope_secondary_id"`.
    #[must_use]
    pub fn to_notify_payload(&self) -> String {
        format!(
            "{}:{}:{}",
            self.scope_type(),
            self.scope_id(),
            self.scope_secondary_id()
        )
    }

    /// Decodes a `pg_notify` payload string produced by `to_notify_payload`.
    pub fn from_notify_payload(s: &str) -> Result<Self, ParseError> {
        let mut parts = s.splitn(3, ':');
        let scope_type =
            parts.next().ok_or_else(|| ParseError("missing scope_type".into()))?;
        let scope_id_str =
            parts.next().ok_or_else(|| ParseError("missing scope_id".into()))?;
        let scope_secondary_id_str = parts.next().unwrap_or("0");

        let scope_id: i64 = scope_id_str
            .parse()
            .map_err(|e| ParseError(format!("invalid scope_id: {e}")))?;
        let scope_secondary_id: i64 = if scope_secondary_id_str.is_empty() {
            0
        } else {
            scope_secondary_id_str.parse().map_err(|e| {
                ParseError(format!("invalid scope_secondary_id: {e}"))
            })?
        };

        let to_u64 = |n: i64| -> Result<u64, ParseError> {
            u64::try_from(n)
                .map_err(|_e| ParseError(format!("negative snowflake: {n}")))
        };

        match scope_type {
            "user" => Ok(Self::User(to_u64(scope_id)?)),
            "guild" => Ok(Self::Guild(to_u64(scope_id)?)),
            "user_in_guild" => {
                Ok(Self::UserInGuild(to_u64(scope_id)?, to_u64(scope_secondary_id)?))
            },
            other => Err(ParseError(format!("unknown scope_type: {other}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_payload_round_trips_all_variants() {
        let cases = [
            EntitlementScope::User(123_456_789),
            EntitlementScope::Guild(987_654_321),
            EntitlementScope::UserInGuild(111_111_111, 222_222_222),
        ];
        for scope in &cases {
            let payload = scope.to_notify_payload();
            let result = EntitlementScope::from_notify_payload(&payload);
            assert!(
                result.is_ok(),
                "round-trip failed for {scope:?}: {:?}",
                result.err()
            );
            if let Ok(decoded) = result {
                assert_eq!(scope, &decoded, "payload was: {payload:?}");
            }
        }
    }

    #[test]
    fn from_notify_payload_rejects_invalid_input() {
        assert!(EntitlementScope::from_notify_payload("badtype:1:0").is_err());
        assert!(EntitlementScope::from_notify_payload("user:notanumber:0").is_err());
        assert!(EntitlementScope::from_notify_payload("").is_err());
    }
}
