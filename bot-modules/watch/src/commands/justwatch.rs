use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

use jellyfin::JellyfinError;
use jellyfin::runtime::JellyfinRuntime;
use jellyfin::transport::jellyseerr::model::MediaType;
use serenity::all::{EditInteractionResponse, ResolvedValue};
use zayden_core::{InvocationCtx, required_option};

use crate::discovery::{providers, resolve};
use crate::embeds;
use crate::error::Result;

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<()> {
    let title: &str = required_option(&mut options, "title")?;

    cx.interaction.defer(&cx.ctx.http).await?;

    let resolved = resolve(runtime, &cx.app.db, title).await?;
    let tmdb_id = resolved
        .tmdb_id()
        .ok_or_else(|| JellyfinError::NoSuchTitle(title.to_owned()))?;

    let region = runtime.seer.region().to_owned();
    let all = fetch_providers(runtime, resolved.kind(), tmdb_id).await?;
    let availability = providers::for_region(&all, &region);

    let mut embed = embeds::media::resolved(&runtime.jellyfin, &resolved);

    if resolved.is_on_server() {
        embed = embed.field(
            "Note",
            "This is already on the server, so you do not need a subscription.",
            false,
        );
    }

    if availability.is_empty() {
        embed = embed.field(
            format!("Streaming in {region}"),
            "Nothing found for your region.",
            false,
        );
    } else {
        if !availability.stream.is_empty() {
            embed = embed.field(
                format!("Streaming in {region}"),
                availability.stream.join(", "),
                false,
            );
        }
        if !availability.buy.is_empty() {
            embed = embed.field("Buy or rent", availability.buy.join(", "), false);
        }
        if let Some(link) = availability.link {
            embed = embed.field("Where", format!("[TMDB]({link})"), false);
        }
    }

    cx.interaction
        .edit_response(&cx.ctx.http, EditInteractionResponse::new().embed(embed))
        .await?;

    Ok(())
}

async fn fetch_providers(
    runtime: &Arc<JellyfinRuntime>,
    kind: MediaType,
    tmdb_id: i32,
) -> Result<Vec<jellyfin::transport::jellyseerr::model::RegionProviders>> {
    let key = (kind.as_str().to_owned(), tmdb_id);

    if let Some(cached) = runtime.caches.watch_providers.get(&key).await {
        return Ok(cached);
    }

    let providers = match kind {
        MediaType::Movie => {
            runtime
                .seer
                .movie(tmdb_id)
                .await
                .map_err(JellyfinError::from)?
                .watch_providers
        },
        MediaType::Tv => {
            runtime
                .seer
                .tv(tmdb_id)
                .await
                .map_err(JellyfinError::from)?
                .watch_providers
        },
    };

    runtime.caches.watch_providers.insert(key, providers.clone()).await;
    Ok(providers)
}
