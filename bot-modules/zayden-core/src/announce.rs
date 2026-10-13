use std::fmt::Write as _;

use serenity::all::{CreateAllowedMentions, MessageBuilder, RoleId};

use crate::snowflake::as_u64;

pub const ROLES: &str = "roles";

#[must_use]
pub fn escape(value: &str) -> String {
    MessageBuilder::new().push_safe(value).build()
}

#[must_use]
pub fn role_mentions(role_ids: &[i64]) -> String {
    role_ids.iter().filter(|id| **id > 0).fold(String::new(), |mut out, id| {
        if !out.is_empty() {
            out.push(' ');
        }
        let _ = write!(out, "<@&{id}>");
        out
    })
}

/// Fills `{name}` tokens in one pass.
///
/// A substituted value is never scanned for further tokens. `{roles}` becomes
/// the role mentions; without it, the mentions lead the message instead.
#[must_use]
pub fn render(template: &str, role_ids: &[i64], vars: &[(&str, &str)]) -> String {
    let mentions = role_mentions(role_ids);
    let mut used_roles = false;

    let mut out = String::with_capacity(template.len());
    let mut rest = template;

    while let Some((before, after)) = rest.split_once('{') {
        out.push_str(before);

        let token = after.split_once('}').and_then(|(key, tail)| {
            if key == ROLES {
                used_roles = true;
                Some((mentions.as_str(), tail))
            } else {
                vars.iter()
                    .find(|(name, _)| *name == key)
                    .map(|(_, value)| (*value, tail))
            }
        });

        match token {
            Some((value, tail)) => {
                out.push_str(value);
                rest = tail;
            },
            None => {
                out.push('{');
                rest = after;
            },
        }
    }
    out.push_str(rest);

    if !used_roles && !mentions.is_empty() {
        out = format!("{mentions} {}", out.trim_start());
    }

    out.trim().to_owned()
}

pub fn allowed_roles(role_ids: &[i64]) -> CreateAllowedMentions<'static> {
    let roles: Vec<RoleId> = role_ids
        .iter()
        .filter(|id| **id > 0)
        .map(|id| RoleId::new(as_u64(*id)))
        .collect();

    CreateAllowedMentions::new()
        .everyone(false)
        .all_users(false)
        .all_roles(false)
        .roles(roles)
}
