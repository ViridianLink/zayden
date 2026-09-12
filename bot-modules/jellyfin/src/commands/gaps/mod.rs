mod letterboxd;
mod serializd;

use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

use serenity::all::{CreateEmbed, EditInteractionResponse, ResolvedValue};
use tracing::warn;
use zayden_core::error::Respond;
use zayden_core::{InvocationCtx, optional_option};

use crate::discovery::gaps::{Candidate, GapReport};
use crate::embeds::COLOUR;
use crate::error::{JellyfinError, Result};
use crate::identity::JellyfinLinkRow;
use crate::jellyscribe;
use crate::runtime::JellyfinRuntime;

const DEFAULT_MIN_RATING: f64 = 3.5;
const MAX_LISTED: usize = 5;

struct Lookup<'a, 'ctx> {
    cx: &'a InvocationCtx<'ctx>,
    runtime: &'a Arc<JellyfinRuntime>,
    linked: Option<&'a JellyfinLinkRow>,
    min_rating: f32,
    settings_url: &'a str,
}

struct Section {
    service: &'static str,
    noun: &'static str,
    username: String,
    report: Result<GapReport>,
    sync: String,
}

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<()> {
    let letterboxd_name: Option<&str> = optional_option(&mut options, "letterboxd");
    let serializd_name: Option<&str> = optional_option(&mut options, "serializd");
    let min_rating = optional_option::<f64, _>(&mut options, "min_rating")
        .unwrap_or(DEFAULT_MIN_RATING);

    cx.interaction.defer(&cx.ctx.http).await?;

    let settings_url = jellyscribe::settings_url(runtime.jellyfin.public_url());
    let linked = JellyfinLinkRow::get(&cx.app.db, cx.interaction.user.id).await?;

    #[expect(
        clippy::cast_possible_truncation,
        reason = "the option is bounded to 0.5..=5.0 stars, well inside f32"
    )]
    let min_rating = min_rating as f32;

    let lookup = Lookup {
        cx,
        runtime,
        linked: linked.as_ref(),
        min_rating,
        settings_url: &settings_url,
    };

    let (films, shows) = futures::join!(
        letterboxd::section(&lookup, letterboxd_name),
        serializd::section(&lookup, serializd_name),
    );

    let sections: Vec<Section> = films.into_iter().chain(shows).collect();
    if sections.is_empty() {
        return Err(JellyfinError::NoDiaryToCheck(settings_url));
    }

    let embed = render(sections)?;

    cx.interaction
        .edit_response(&cx.ctx.http, EditInteractionResponse::new().embed(embed))
        .await?;

    Ok(())
}

fn render(sections: Vec<Section>) -> Result<CreateEmbed<'static>> {
    let mut embed = CreateEmbed::new().title("Gaps in the library").colour(COLOUR);
    let mut sync = Vec::with_capacity(sections.len());
    let mut first_error = None;
    let mut read_any = false;

    for section in sections {
        let body = match section.report {
            Ok(report) => {
                read_any = true;
                summary(&report, section.noun)
            },
            Err(error) => {
                let message = error.user_message().map_or_else(
                    || {
                        warn!(error = ?error, service = section.service, "could not read a diary");
                        format!("I could not read that {} diary just now.", section.service)
                    },
                    Cow::into_owned,
                );
                first_error.get_or_insert(error);
                message
            },
        };

        embed = embed.field(
            format!(
                "Missing {}s — {} {}",
                section.noun, section.service, section.username
            ),
            body,
            false,
        );
        sync.push(format!("**{}:** {}", section.service, section.sync));
    }

    // With nothing read at all, the error reply says more than an empty embed.
    if !read_any && let Some(error) = first_error {
        return Err(error);
    }

    Ok(embed.field("Syncing the other way", sync.join("\n\n"), false))
}

fn summary(report: &GapReport, noun: &str) -> String {
    let missing = if report.missing.is_empty() {
        format!("The server has every {noun} you rated that highly.")
    } else {
        report.missing.iter().map(line).collect::<Vec<_>>().join("\n")
    };

    let approximate = if report.matched_by_title > 0 {
        " Title matches are approximate."
    } else {
        ""
    };

    format!(
        "{missing}\n\n*{} read: {} matched by TMDB id, {} by title only, {} not \
         found.{approximate}*",
        report.total,
        report.matched_by_id,
        report.matched_by_title,
        report.unmatched,
    )
}

fn line(candidate: &Candidate) -> String {
    let rating = candidate.rating.map_or_else(String::new, |r| format!(" — {r}★"));

    candidate.year.map_or_else(
        || format!("**{}**{rating}", candidate.title),
        |year| format!("**{}** ({year}){rating}", candidate.title),
    )
}
