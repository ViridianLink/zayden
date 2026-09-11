pub mod gaps;
pub mod sync;

use std::sync::Arc;

use jellyfin::runtime::JellyfinRuntime;
use zayden_core::{InvocationCtx, parse_options, parse_subcommand};

use crate::error::{Result, WatchError};

pub async fn run(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
) -> Result<()> {
    let (_, group_options) = parse_subcommand(cx.interaction.data.options())?;
    let (name, leaf_options) = parse_subcommand(group_options)?;
    let options = parse_options(leaf_options);

    match name {
        "gaps" => gaps::run(cx, runtime, options).await,
        "sync" => sync::run(cx, runtime, options).await,
        _ => Err(WatchError::UnknownSubcommand(name.to_string())),
    }
}
