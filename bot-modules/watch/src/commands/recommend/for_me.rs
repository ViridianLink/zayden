use std::collections::HashMap;
use std::sync::Arc;

use jellyfin::games::question::history;
use jellyfin::identity::JellyfinLinkRow;
use jellyfin::index::LibraryItemRow;
use jellyfin::runtime::JellyfinRuntime;
use serenity::all::{CreateEmbed, EditInteractionResponse};
use zayden_core::InvocationCtx;

use crate::embeds::COLOUR;
use crate::error::Result;

const SUGGESTIONS: usize = 8;

pub async fn run(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
) -> Result<()> {
    cx.interaction.defer(&cx.ctx.http).await?;

    let link = JellyfinLinkRow::require(&cx.app.db, cx.interaction.user.id).await?;
    let plays = history::own_history(runtime, &link.jellyfin_user_id).await?;

    if plays.is_empty() {
        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new()
                    .content("You have no watch history on the server yet."),
            )
            .await?;
        return Ok(());
    }

    // Favourite genres come from what has actually been played, then the
    // suggestions are drawn from the server's own catalogue: recommending
    // something nobody can watch would be useless.
    let mut genre_counts: HashMap<String, usize> = HashMap::new();
    let mut seen = Vec::new();

    for play in &plays {
        seen.push(play.item_id.clone());

        if let Some(row) = LibraryItemRow::by_id(&cx.app.db, &play.item_id).await? {
            for genre in row.genres {
                *genre_counts.entry(genre).or_default() += 1;
            }
        }
    }

    let mut ranked: Vec<(String, usize)> = genre_counts.into_iter().collect();
    ranked.sort_by_key(|(_, count)| std::cmp::Reverse(*count));

    let favourites: Vec<String> =
        ranked.iter().take(3).map(|(genre, _)| genre.clone()).collect();

    let candidates = LibraryItemRow::hidden_gems(&cx.app.db, 0.0, 200).await?;
    let picks: Vec<&LibraryItemRow> = candidates
        .iter()
        .filter(|row| !seen.contains(&row.item_id))
        .filter(|row| {
            favourites.is_empty()
                || row.genres.iter().any(|g| favourites.contains(g))
        })
        .take(SUGGESTIONS)
        .collect();

    let body = if picks.is_empty() {
        "Nothing new in your usual genres — try `/watch recommend hidden-gem`."
            .to_owned()
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

    let embed = CreateEmbed::new()
        .title("Picked for you")
        .colour(COLOUR)
        .description(body)
        .field(
            "Based on",
            if favourites.is_empty() {
                "your watch history".to_owned()
            } else {
                favourites.join(", ")
            },
            false,
        );

    cx.interaction
        .edit_response(&cx.ctx.http, EditInteractionResponse::new().embed(embed))
        .await?;

    Ok(())
}
