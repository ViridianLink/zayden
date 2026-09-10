use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

use jellyfin::runtime::JellyfinRuntime;
use serenity::all::{EditInteractionResponse, ResolvedValue};
use zayden_core::{InvocationCtx, required_option};

use crate::discovery::vibe;
use crate::embeds;
use crate::error::Result;

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<()> {
    let description: &str = required_option(&mut options, "description")?;

    cx.interaction.defer(&cx.ctx.http).await?;

    let facets = vibe::facets(&cx.app, description).await?;
    let filters = vibe::to_filters(runtime, &facets).await?;
    let (results, relaxed) = vibe::search(runtime, filters).await?;

    let mut note = format!(
        "Read as: {}{}",
        facets.genres.join(", "),
        if facets.keywords.is_empty() {
            String::new()
        } else {
            format!(" · {}", facets.keywords.join(", "))
        }
    );

    if relaxed {
        note.push_str(
            "\nThat was too specific to match much, so I loosened it a little.",
        );
    }

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new().embed(embeds::media::list(
                "That vibe",
                &results,
                Some(&note),
            )),
        )
        .await?;

    Ok(())
}
