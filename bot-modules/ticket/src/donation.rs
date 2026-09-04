use std::collections::HashMap;
use std::fmt::Write as _;

use futures::{StreamExt, TryStreamExt};
use serenity::all::{GuildId, Http, Mentionable, RoleId, ThreadId, UserId};
use sqlx::PgPool;

use crate::Result;
use crate::helper_links::HelperLinks;

const HELPER_SCAN_LIMIT: usize = 500;

pub async fn message(
    http: &Http,
    pool: &PgPool,
    thread_id: ThreadId,
    guild_id: GuildId,
    helper_roles: &[RoleId],
) -> Result<Option<String>> {
    if helper_roles.is_empty() {
        return Ok(None);
    }

    let links = HelperLinks::map(pool, guild_id).await?;

    if links.is_empty() {
        return Ok(None);
    }

    let helpers = scan_helpers(http, thread_id, helper_roles, &links).await?;

    if helpers.is_empty() {
        return Ok(None);
    }

    Ok(Some(format_message(&helpers)))
}

async fn scan_helpers(
    http: &Http,
    thread_id: ThreadId,
    helper_roles: &[RoleId],
    links: &HashMap<UserId, String>,
) -> Result<Vec<(UserId, String)>> {
    let helpers = thread_id
        .widen()
        .messages_iter(http)
        .take(HELPER_SCAN_LIMIT)
        .try_fold(HashMap::new(), async |mut helpers, m| {
            let Some(member) = m.member else { return Ok(helpers) };

            if !member.roles.iter().any(|role| helper_roles.contains(role)) {
                return Ok(helpers);
            }

            if let Some(link) = links.get(&m.author.id) {
                helpers.insert(m.author.id, link.clone());
            }

            Ok(helpers)
        })
        .await?;

    let mut helpers = helpers.into_iter().collect::<Vec<_>>();
    helpers.sort_unstable_by_key(|(id, _)| *id);

    Ok(helpers)
}

#[must_use]
pub fn format_message(helpers: &[(UserId, String)]) -> String {
    let mut reply =
        String::from("If this helped, consider supporting the people who did:");

    for (id, link) in helpers {
        let _ = write!(reply, "\n{}: {link}", id.mention());
    }

    reply
}
