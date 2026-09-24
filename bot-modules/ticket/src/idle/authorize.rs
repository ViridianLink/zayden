use serenity::all::{RoleId, UserId};

#[must_use]
pub fn may_act(
    actor: UserId,
    op: Option<UserId>,
    actor_roles: &[RoleId],
    support_roles: &[RoleId],
    manage_messages: bool,
) -> bool {
    op == Some(actor)
        || manage_messages
        || actor_roles.iter().any(|role| support_roles.contains(role))
}
