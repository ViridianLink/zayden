use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

use jellyfin::runtime::JellyfinRuntime;
use serenity::all::{CreateEmbed, EditInteractionResponse, ResolvedValue};
use zayden_core::{InvocationCtx, required_option};

use crate::discovery::resolve;
use crate::discovery::warnings::{self, Severity};
use crate::embeds::COLOUR;
use crate::error::Result;

const MAX_LISTED: usize = 12;

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<()> {
    let title: &str = required_option(&mut options, "title")?;

    cx.interaction.defer(&cx.ctx.http).await?;

    let resolved = resolve(runtime, &cx.app.db, title).await?;
    let found =
        warnings::lookup(&cx.app.http, runtime.dddie_api_key(), &resolved.title())
            .await?;

    let body = if found.is_empty() {
        "Nothing reported.".to_owned()
    } else {
        found
            .iter()
            .take(MAX_LISTED)
            .map(|w| {
                format!(
                    "{} {} ({} of {})",
                    w.severity.emoji(),
                    w.topic,
                    w.yes,
                    w.yes + w.no
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let embed = CreateEmbed::new()
        .title(format!("Content warnings: {}", resolved.title()))
        .colour(COLOUR)
        .description(body)
        .field(
            "Key",
            format!(
                "{} mild · {} moderate · {} severe — community reported, not \
                 exhaustive.",
                Severity::Mild.emoji(),
                Severity::Moderate.emoji(),
                Severity::Severe.emoji()
            ),
            false,
        );

    cx.interaction
        .edit_response(&cx.ctx.http, EditInteractionResponse::new().embed(embed))
        .await?;

    Ok(())
}
