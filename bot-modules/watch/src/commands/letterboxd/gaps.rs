use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

use jellyfin::JellyfinError;
use jellyfin::identity::JellyfinLinkRow;
use jellyfin::jellyscribe::{self, Linked};
use jellyfin::runtime::JellyfinRuntime;
use serenity::all::{CreateEmbed, EditInteractionResponse, ResolvedValue};
use tracing::warn;
use zayden_core::{InvocationCtx, optional_option};

use crate::discovery::letterboxd;
use crate::embeds::COLOUR;
use crate::error::Result;

const DEFAULT_MIN_RATING: f64 = 3.5;
const MAX_LISTED: usize = 5;

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<()> {
    let supplied: Option<&str> = optional_option(&mut options, "username");
    let min_rating = optional_option::<f64, _>(&mut options, "min_rating")
        .unwrap_or(DEFAULT_MIN_RATING);

    cx.interaction.defer(&cx.ctx.http).await?;

    let user_id = cx.interaction.user.id;
    let settings_url = jellyscribe::settings_url(runtime.jellyfin.base_url());

    let linked = JellyfinLinkRow::get(&cx.app.db, user_id).await?;
    let account = match linked.as_ref() {
        Some(row) => jellyscribe::account(&runtime.jellyfin, &row.jellyfin_user_id)
            .await
            .inspect_err(|e| {
                warn!(error = ?e, %user_id, "could not read Jellyscribe accounts");
            })
            .ok()
            .flatten(),
        None => None,
    };

    let stored = linked.as_ref().and_then(|row| row.letterboxd_username.as_deref());
    let known = account.as_ref().and_then(|a| a.letterboxd_username.as_deref());

    let username =
        supplied.or(stored).or(known).map(str::to_owned).ok_or_else(|| {
            JellyfinError::NoLetterboxdFeed(format!(
                "nobody — pass a username and I will register it with \
                 Jellyscribe for you, or set one up yourself at {settings_url}"
            ))
        })?;

    // Remember an explicitly supplied name so the option is optional next time.
    if supplied.is_some() {
        JellyfinLinkRow::set_letterboxd(&cx.app.db, user_id, Some(&username))
            .await
            .ok();
    }

    let registration = register(runtime, linked.as_ref(), known, &username).await;

    let entries = letterboxd::fetch(&cx.app.http, &username).await?;
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a Letterboxd rating is 0.5..=5.0, so f64 -> f32 is exact"
    )]
    let report = letterboxd::cross_reference(
        &cx.app.db,
        &entries,
        min_rating as f32,
        MAX_LISTED,
    )
    .await?;

    let missing = if report.missing.is_empty() {
        "The server has everything you rated that highly.".to_owned()
    } else {
        report
            .missing
            .iter()
            .map(|e| {
                let rating =
                    e.rating.map_or_else(String::new, |r| format!(" — {r}★"));
                e.year.map_or_else(
                    || format!("**{}**{rating}", e.title),
                    |year| format!("**{}** ({year}){rating}", e.title),
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let enabled = account.as_ref().is_some_and(|a| a.enabled);
    let sync = sync_status(registration, &username, enabled, &settings_url);

    let embed = CreateEmbed::new()
        .title(format!("Letterboxd gaps for {username}"))
        .colour(COLOUR)
        .description(missing)
        .field(
            "Matching",
            format!(
                "{} entries read: {} matched by TMDB id, {} by title only, {} \
                 not found. Title matches are approximate.",
                report.total,
                report.matched_by_id,
                report.matched_by_title,
                report.unmatched,
            ),
            false,
        )
        .field("Syncing the other way", sync, false);

    cx.interaction
        .edit_response(&cx.ctx.http, EditInteractionResponse::new().embed(embed))
        .await?;

    Ok(())
}

async fn register(
    runtime: &Arc<JellyfinRuntime>,
    linked: Option<&JellyfinLinkRow>,
    known: Option<&str>,
    username: &str,
) -> Option<Linked> {
    if known.is_some_and(|name| name.eq_ignore_ascii_case(username)) {
        return Some(Linked::Unchanged);
    }

    let row = linked?;

    jellyscribe::link(&runtime.jellyfin, &row.jellyfin_user_id, username)
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
