use serenity::all::{RoleId, UserId};

#[must_use]
pub fn may_act(
    presser: UserId,
    op: UserId,
    presser_roles: &[RoleId],
    support_roles: &[RoleId],
    manage_messages: bool,
) -> bool {
    presser == op
        || manage_messages
        || presser_roles.iter().any(|role| support_roles.contains(role))
}
