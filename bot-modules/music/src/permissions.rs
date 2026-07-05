use serenity::all::{Permissions, RoleId, UserId};
use zayden_core::as_u64;

#[must_use]
pub fn is_privileged(
    member_roles: &[RoleId],
    member_permissions: Option<Permissions>,
    dj_role_id: Option<i64>,
) -> bool {
    if member_permissions.is_some_and(Permissions::manage_guild) {
        return true;
    }

    dj_role_id.is_none_or(|id| member_roles.contains(&RoleId::new(as_u64(id))))
}

#[must_use]
pub const fn can_manage_track(
    privileged: bool,
    requester: UserId,
    invoker: UserId,
) -> bool {
    privileged || requester.get() == invoker.get()
}

#[must_use]
pub fn vote_threshold(non_bot_listeners: usize) -> usize {
    non_bot_listeners.div_ceil(2).max(1)
}
