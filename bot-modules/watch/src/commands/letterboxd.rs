use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

use jellyfin::JellyfinError;
use jellyfin::identity::JellyfinLinkRow;
use jellyfin::runtime::JellyfinRuntime;
use serenity::all::{CreateEmbed, EditInteractionResponse, ResolvedValue};
use zayden_core::{InvocationCtx, optional_option};

use crate::discovery::letterboxd;
use crate::embeds::COLOUR;
use crate::error::Result;

const DEFAULT_MIN_RATING: f64 = 3.5;
const MAX_LISTED: usize = 5;

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    _runtime: &Arc<JellyfinRuntime>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<()> {
    let supplied: Option<&str> = optional_option(&mut options, "username");
    let min_rating = optional_option::<f64, _>(&mut options, "min_rating")
        .unwrap_or(DEFAULT_MIN_RATING);

    cx.interaction.defer(&cx.ctx.http).await?;

    let user_id = cx.interaction.user.id;
    let stored = JellyfinLinkRow::get(&cx.app.db, user_id)
        .await?
        .and_then(|row| row.letterboxd_username);

    let username = supplied.map(str::to_owned).or(stored).ok_or_else(|| {
        JellyfinError::NoLetterboxdFeed(
            "nobody — pass a username, or set one up in Jellyfin's \
                 Jellyscribe settings"
                .to_owned(),
        )
    })?;

    // Remember an explicitly supplied name so the option is optional next time.
    if supplied.is_some() {
        JellyfinLinkRow::set_letterboxd(&cx.app.db, user_id, Some(&username))
            .await
            .ok();
    }

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
        .field(
            "Syncing the other way",
            "Pushing your Jellyfin watches *to* Letterboxd is handled by the \
             Jellyscribe plugin in Jellyfin itself, not by me.",
            false,
        );

    cx.interaction
        .edit_response(&cx.ctx.http, EditInteractionResponse::new().embed(embed))
        .await?;

    Ok(())
}
