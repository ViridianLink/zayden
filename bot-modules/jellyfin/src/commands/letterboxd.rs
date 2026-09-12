use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::Arc;

use serenity::all::{CreateEmbed, EditInteractionResponse, ResolvedValue};
use tracing::warn;
use zayden_core::{InvocationCtx, required_option};

use crate::embeds::COLOUR;
use crate::error::Result;
use crate::identity::JellyfinLinkRow;
use crate::jellyscribe::letterboxd as jellyscribe;
use crate::runtime::JellyfinRuntime;

const NEW_ACCOUNT_DEFAULTS: &str = "Enabled, favourites liked, primary account, \
     watchlist synced to the library, auto-request via Seerr, diary imported as \
     watched, already-synced skipped. Date filter, Seerr backfill, watchlist \
     mirroring and stop-on-failure are off. Change any of them on your \
     Jellyscribe page.";

pub async fn run<S: BuildHasher>(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
    mut options: HashMap<&str, ResolvedValue<'_>, S>,
) -> Result<()> {
    let username: &str = required_option(&mut options, "username")?;
    let password: &str = required_option(&mut options, "password")?;

    cx.interaction.defer_ephemeral(&cx.ctx.http).await?;

    let user_id = cx.interaction.user.id;
    let row = JellyfinLinkRow::require(&cx.app.db, user_id).await?;

    jellyscribe::test_connection(&runtime.jellyfin, username, password).await?;

    let created = jellyscribe::configure(
        &runtime.jellyfin,
        &row.jellyfin_user_id,
        username,
        password,
    )
    .await?;

    JellyfinLinkRow::set_letterboxd(&cx.app.db, user_id, Some(username)).await.ok();

    let started = jellyscribe::start_sync(&runtime.jellyfin)
        .await
        .inspect_err(|e| {
            warn!(error = ?e, %user_id, "could not start the Jellyscribe sync task");
        })
        .unwrap_or(false);

    let embed = embed(username, created, started);

    cx.interaction
        .edit_response(&cx.ctx.http, EditInteractionResponse::new().embed(embed))
        .await?;

    Ok(())
}

fn embed(username: &str, created: bool, started: bool) -> CreateEmbed<'static> {
    let account = if created { "Added" } else { "Updated" };
    let sync = if started {
        "A sync is running now; your recent Jellyfin watches should reach your \
         diary shortly."
    } else {
        "I could not start a sync just now, so the plugin's daily run will pick \
         it up instead."
    };

    let embed = CreateEmbed::new()
        .title("Letterboxd sync is set up")
        .colour(COLOUR)
        .description(format!(
            "{account} **{username}** in the Jellyscribe plugin and switched it \
             on. Letterboxd accepted the login, and your password went straight \
             to the plugin — I do not keep it.\n\n{sync}"
        ));

    if created {
        embed.field("Defaults", NEW_ACCOUNT_DEFAULTS, false)
    } else {
        embed
    }
}
