use std::collections::HashMap;

use serenity::all::{Permissions, RoleId, UserId};
use zayden_app::config::HoneypotSettingsRow;
use zayden_core::as_u64;

#[derive(Debug, Clone)]
pub struct GuildFacts {
    pub owner_id: UserId,
    pub role_perms: HashMap<RoleId, Permissions>,
    pub everyone_role: RoleId,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExemptionPolicy {
    pub exempt_admins: bool,
    pub exempt_role_id: Option<RoleId>,
}

impl From<&HoneypotSettingsRow> for ExemptionPolicy {
    fn from(row: &HoneypotSettingsRow) -> Self {
        Self {
            exempt_admins: row.exempt_admins,
            exempt_role_id: row.exempt_role_id.map(|id| RoleId::new(as_u64(id))),
        }
    }
}

#[must_use]
pub fn guild_permissions(
    member_roles: &[RoleId],
    facts: &GuildFacts,
) -> Permissions {
    let mut perms =
        facts.role_perms.get(&facts.everyone_role).copied().unwrap_or_default();

    for role in member_roles {
        if let Some(role_perms) = facts.role_perms.get(role) {
            perms |= *role_perms;
        }
    }

    perms
}

#[must_use]
pub fn is_staff(perms: Permissions) -> bool {
    perms.administrator() || perms.manage_guild()
}

#[must_use]
pub fn is_exempt(
    author_id: UserId,
    member_roles: &[RoleId],
    facts: &GuildFacts,
    policy: &ExemptionPolicy,
) -> bool {
    if author_id == facts.owner_id {
        return true;
    }

    if policy.exempt_role_id.is_some_and(|role| member_roles.contains(&role)) {
        return true;
    }

    policy.exempt_admins && is_staff(guild_permissions(member_roles, facts))
}
