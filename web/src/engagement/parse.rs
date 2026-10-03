use std::num::NonZeroU64;

use greetings::GuildId;

use super::error::EngagementError;
use crate::auth::AuthError;

pub(crate) fn parse_snowflake(
    label: &'static str,
    raw: &str,
) -> Result<u64, EngagementError> {
    raw.trim().parse::<u64>().map_err(|_e| EngagementError::Invalid(label))
}

/// A snowflake that is a usable Discord id: zero and `u64::MAX` are no
/// snowflakes.
pub(crate) fn usable_id(
    label: &'static str,
    id: u64,
) -> Result<NonZeroU64, EngagementError> {
    NonZeroU64::new(id)
        .filter(|id| id.get() != u64::MAX)
        .ok_or(EngagementError::Invalid(label))
}

/// The guild as Discord's id type. Zero and `u64::MAX` name no guild.
pub(crate) fn guild_ref(guild_id: i64) -> Result<GuildId, EngagementError> {
    usable_id("guild id", guild_id.cast_unsigned())
        .map(|id| GuildId::new(id.get()))
        .map_err(|_e| AuthError::InvalidGuildId.into())
}
