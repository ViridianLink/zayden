use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

use jellyfin::JellyfinError;
use jellyfin::index::LibraryItemRow;
use jellyfin::runtime::JellyfinRuntime;
use serenity::all::{CreateEmbed, EditInteractionResponse, ResolvedValue};
use zayden_core::{InvocationCtx, optional_option};

use crate::embeds::COLOUR;
use crate::error::Result;

const DEFAULT_MIN_RATING: f64 = 7.5;
const CANDIDATES: i64 = 200;
const SHOWN: usize = 8;
const QUIET_PLAYS: i64 = 2;

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<()> {
    let min_rating = optional_option::<f64, _>(&mut options, "min_rating")
        .unwrap_or(DEFAULT_MIN_RATING);
    let genre: Option<&str> = optional_option(&mut options, "genre");

    cx.interaction.defer(&cx.ctx.http).await?;

    #[expect(
        clippy::cast_possible_truncation,
        reason = "a TMDB rating is 0.0..=10.0, so f64 -> f32 is exact"
    )]
    let candidates =
        LibraryItemRow::hidden_gems(&cx.app.db, min_rating as f32, CANDIDATES)
            .await?;

    if candidates.is_empty() {
        return Err(JellyfinError::EmptyIndex.into());
    }

    let counts = play_counts(runtime).await?;

    let picks: Vec<&LibraryItemRow> = candidates
        .iter()
        .filter(|row| {
            genre.is_none_or(|g| {
                row.genres.iter().any(|item| item.eq_ignore_ascii_case(g))
            })
        })
        .filter(|row| {
            counts
                .iter()
                .find(|c| c.item_id == row.item_id)
                .is_none_or(|c| c.plays <= QUIET_PLAYS)
        })
        .take(SHOWN)
        .collect();

    let body = if picks.is_empty() {
        "Everything that highly rated has already been watched here.".to_owned()
    } else {
        picks
            .iter()
            .map(|row| {
                let rating = row
                    .community_rating
                    .map_or_else(String::new, |r| format!(" — {r:.1}"));
                format!(
                    "**{}**{rating} — [open]({})",
                    row.name,
                    runtime.jellyfin.item_url(&row.item_id)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new().embed(
                CreateEmbed::new()
                    .title("Hidden gems")
                    .colour(COLOUR)
                    .description(body)
                    .field(
                        "Criteria",
                        format!(
                            "Rated {min_rating:.1} or better, watched {QUIET_PLAYS} \
                             times or fewer on this server."
                        ),
                        false,
                    ),
            ),
        )
        .await?;

    Ok(())
}

async fn play_counts(
    runtime: &Arc<JellyfinRuntime>,
) -> Result<Vec<jellyfin::transport::playback::model::ItemPlayCount>> {
    if let Some(cached) = runtime.caches.play_counts.get(&()).await {
        return Ok(cached);
    }

    let counts =
        runtime.playback.item_play_counts().await.map_err(JellyfinError::from)?;

    runtime.caches.play_counts.insert((), counts.clone()).await;
    Ok(counts)
}
