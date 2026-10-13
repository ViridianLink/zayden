use topcoat::context::Cx;

use super::error::GuildError;
use super::form::form_args;
use super::parse::parse_role;
use crate::auth::GuildIds;

form_args! {
    PingRoleForm { guild, role_id }
}

pub const MAX_MESSAGE_CHARS: usize = 1500;
pub const MAX_PING_ROLES: usize = 10;

/// An empty message means the provider's default.
pub fn parse_message(raw: &str) -> Result<Option<&str>, GuildError> {
    let message = raw.trim();

    if message.chars().count() > MAX_MESSAGE_CHARS {
        return Err(GuildError::MessageTooLong(MAX_MESSAGE_CHARS));
    }

    Ok((!message.is_empty()).then_some(message))
}

pub(crate) async fn new_ping_role(
    cx: &Cx,
    guild_id: i64,
    configured: &[i64],
    raw: &str,
) -> Result<i64, GuildError> {
    let role = parse_role(raw)?.cast_signed();

    // The @everyone role shares the guild's id and is never pingable here.
    if role == guild_id {
        return Err(GuildError::InvalidRole);
    }
    if configured.contains(&role) {
        return Err(GuildError::DuplicatePingRole);
    }
    if configured.len() >= MAX_PING_ROLES {
        return Err(GuildError::TooManyPingRoles(MAX_PING_ROLES));
    }

    GuildIds::default().role(Some(role)).ensure_in(cx, guild_id).await?;

    Ok(role)
}

/// The column bound for the append, which refuses a role once the stored list
/// is full even when two adds race past the check in [`new_ping_role`].
pub(crate) fn ping_role_limit() -> i32 {
    i32::try_from(MAX_PING_ROLES).unwrap_or(i32::MAX)
}

/// Why an append changed nothing, judged from the list stored afterwards.
pub(crate) fn refused_ping_role(stored: Option<&[i64]>, role: i64) -> GuildError {
    match stored {
        None => GuildError::NoAnnounceChannel,
        Some(roles) if roles.contains(&role) => GuildError::DuplicatePingRole,
        Some(_) => GuildError::TooManyPingRoles(MAX_PING_ROLES),
    }
}
