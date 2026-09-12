use tracing::warn;

use super::{Lookup, MAX_LISTED, Section};
use crate::discovery::gaps::{self, GapReport};
use crate::discovery::serializd::{self as diary, DiaryEntry};
use crate::error::Result;
use crate::identity::JellyfinLinkRow;
use crate::jellyscribe::serializd::{self as plugin, Account};

pub(super) async fn section(
    lookup: &Lookup<'_, '_>,
    supplied: Option<&str>,
) -> Option<Section> {
    let account = match lookup.linked {
        Some(row) => plugin::account(&lookup.runtime.jellyfin, &row.jellyfin_user_id)
            .await
            .inspect_err(|e| {
                warn!(error = ?e, user_id = row.user_id, "could not read Jellyscribe accounts");
            })
            .ok()
            .flatten(),
        None => None,
    };

    let stored = lookup.linked.and_then(|row| row.serializd_username.as_deref());
    let known = account.as_ref().and_then(|a| a.username.as_deref());

    let username = supplied
        .or(stored)
        .or(known)
        .map(str::trim)
        .filter(|name| !name.is_empty())?
        .to_owned();

    // Remember an explicitly supplied name so the option is optional next time.
    if supplied.is_some() {
        JellyfinLinkRow::set_serializd(
            &lookup.cx.app.db,
            lookup.cx.interaction.user.id,
            Some(&username),
        )
        .await
        .ok();
    }

    Some(Section {
        service: "Serializd",
        noun: "show",
        sync: sync_status(account.as_ref(), &username, lookup.settings_url),
        report: report(lookup, &username).await,
        username,
    })
}

async fn report(lookup: &Lookup<'_, '_>, username: &str) -> Result<GapReport> {
    let entries = diary::fetch(&lookup.cx.app.http, username).await?;

    gaps::cross_reference(
        &lookup.cx.app.db,
        "Series",
        entries.iter().map(DiaryEntry::candidate).collect(),
        lookup.min_rating,
        MAX_LISTED,
    )
    .await
}

fn sync_status(
    account: Option<&Account>,
    username: &str,
    settings_url: &str,
) -> String {
    match account {
        Some(Account { enabled: true, username: synced }) => format!(
            "Jellyscribe is already logging your Jellyfin episodes to **{}**.",
            synced.as_deref().unwrap_or(username)
        ),
        Some(_) => format!(
            "Jellyscribe has a Serializd login for you but it is switched off. \
             Run `/jellyfin serializd` or finish it at {settings_url}."
        ),
        None => format!(
            "Run `/jellyfin serializd` with your Serializd login, or add it at \
             {settings_url}, and Jellyscribe will log your Jellyfin episodes there."
        ),
    }
}
