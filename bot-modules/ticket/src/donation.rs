use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use futures::{StreamExt, TryStreamExt};
use serenity::all::{GuildId, Http, Mentionable, RoleId, ThreadId, UserId};
use sqlx::PgPool;
use tracing::warn;

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

    let helpers =
        scan_helpers(http, guild_id, thread_id, helper_roles, &links).await?;

    if helpers.is_empty() {
        return Ok(None);
    }

    Ok(Some(format_message(&helpers)))
}

async fn scan_helpers(
    http: &Http,
    guild_id: GuildId,
    thread_id: ThreadId,
    helper_roles: &[RoleId],
    links: &HashMap<UserId, String>,
) -> Result<Vec<(UserId, String)>> {
    let candidates = speakers(http, thread_id, links).await?;

    let mut helpers = Vec::with_capacity(candidates.len());

    for id in candidates {
        // `Message::member` is only populated on gateway events, so the roles
        // have to be read back per author rather than off the history fetch.
        let member = match guild_id.member(http, id).await {
            Ok(member) => member,
            Err(e) => {
                warn!(error = ?e, %guild_id, user_id = %id, "could not read helper roles");
                continue;
            },
        };

        if member.roles.iter().any(|role| helper_roles.contains(role))
            && let Some(link) = links.get(&id)
        {
            helpers.push((id, link.clone()));
        }
    }

    helpers.sort_unstable_by_key(|(id, _)| *id);

    Ok(helpers)
}

async fn speakers(
    http: &Http,
    thread_id: ThreadId,
    links: &HashMap<UserId, String>,
) -> Result<HashSet<UserId>> {
    thread_id
        .widen()
        .messages_iter(http)
        .take(HELPER_SCAN_LIMIT)
        .try_fold(HashSet::new(), async |mut speakers, m| {
            if !m.author.bot() && links.contains_key(&m.author.id) {
                speakers.insert(m.author.id);
            }

            Ok(speakers)
        })
        .await
        .map_err(Into::into)
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
