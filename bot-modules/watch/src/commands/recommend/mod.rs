pub mod because;
pub mod for_me;
pub mod hidden_gem;
pub mod mix;
pub mod trending;
pub mod vibe;

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
        "trending" => trending::run(cx, runtime).await,
        "because" => because::run(cx, runtime, options).await,
        "mix" => mix::run(cx, runtime, options).await,
        "for-me" => for_me::run(cx, runtime).await,
        "hidden-gem" => hidden_gem::run(cx, runtime, options).await,
        "vibe" => vibe::run(cx, runtime, options).await,
        _ => Err(WatchError::UnknownSubcommand(name.to_string())),
    }
}
