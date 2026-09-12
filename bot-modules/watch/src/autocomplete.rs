use std::sync::Arc;

use jellyfin::autocomplete::{
    local_choices,
    remote_choices,
    respond,
    subcommand_path,
};
use jellyfin::runtime::JellyfinRuntime;
use zayden_core::AutocompleteCtx;

use crate::error::Result;

pub async fn run(
    cx: &AutocompleteCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
) -> Result<()> {
    let Some(focused) = cx.interaction.data.autocomplete() else {
        return Ok(());
    };

    let path = subcommand_path(cx);
    let query = focused.value;

    let choices = match path.as_deref() {
        Some("party create") => local_choices(cx, query, None).await?,
        Some("binge") => local_choices(cx, query, Some("Series")).await?,
        _ => remote_choices(cx, runtime, query).await?,
    };

    respond(cx, choices).await?;

    Ok(())
}
