use tracing::warn;

use super::{Lookup, MAX_LISTED, Section};
use crate::discovery::gaps::{self, GapReport};
use crate::discovery::letterboxd::{self as diary, DiaryEntry};
use crate::error::Result;
use crate::identity::JellyfinLinkRow;
use crate::jellyscribe::letterboxd::{self as plugin, Linked};

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

    let stored = lookup.linked.and_then(|row| row.letterboxd_username.as_deref());
    let known = account.as_ref().and_then(|a| a.username.as_deref());

    let username = supplied
        .or(stored)
        .or(known)
        .map(str::trim)
        .filter(|name| !name.is_empty())?
        .to_owned();

    // Remember an explicitly supplied name so the option is optional next time.
    if supplied.is_some() {
        JellyfinLinkRow::set_letterboxd(
            &lookup.cx.app.db,
            lookup.cx.interaction.user.id,
            Some(&username),
        )
        .await
        .ok();
    }

    let registration = register(lookup, known, &username).await;
    let enabled = account.as_ref().is_some_and(|a| a.enabled);

    Some(Section {
        service: "Letterboxd",
        noun: "film",
        sync: sync_status(registration, &username, enabled, lookup.settings_url),
        report: report(lookup, &username).await,
        username,
    })
}

async fn report(lookup: &Lookup<'_, '_>, username: &str) -> Result<GapReport> {
    let entries = diary::fetch(&lookup.cx.app.http, username).await?;

    gaps::cross_reference(
        &lookup.cx.app.db,
        "Movie",
        entries.iter().map(DiaryEntry::candidate).collect(),
        lookup.min_rating,
        MAX_LISTED,
    )
    .await
}

async fn register(
    lookup: &Lookup<'_, '_>,
    known: Option<&str>,
    username: &str,
) -> Option<Linked> {
    if known.is_some_and(|name| name.eq_ignore_ascii_case(username)) {
        return Some(Linked::Unchanged);
    }

    let row = lookup.linked?;

    plugin::link(&lookup.runtime.jellyfin, &row.jellyfin_user_id, username)
        .await
        .inspect_err(|e| {
            warn!(error = ?e, user_id = row.user_id, "could not reach Jellyscribe");
        })
        .ok()
}

fn sync_status(
    registration: Option<Linked>,
    username: &str,
    enabled: bool,
    settings_url: &str,
) -> String {
    match registration {
        Some(Linked::Created) => format!(
            "I registered **{username}** with the Jellyscribe plugin for you. \
             Add your Letterboxd password at {settings_url} and tick Enabled, \
             and your Jellyfin watches will start showing up in your diary."
        ),
        Some(Linked::Unchanged) if enabled => format!(
            "Jellyscribe is already pushing your Jellyfin watches to \
             **{username}**."
        ),
        Some(Linked::Unchanged) => format!(
            "Jellyscribe knows **{username}** but is switched off for you. \
             Finish it at {settings_url}."
        ),
        Some(Linked::Conflict(other)) => format!(
            "Jellyscribe already syncs this Jellyfin account to **{other}**, so \
             I left it alone. Change it at {settings_url}."
        ),
        None => format!(
            "Run `/jellyfin link` first and I can set the Jellyscribe plugin up \
             for you. Otherwise it is at {settings_url}."
        ),
    }
}
