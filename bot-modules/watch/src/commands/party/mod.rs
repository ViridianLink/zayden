pub mod cancel;
pub mod create;
pub mod info;
pub mod list;

use std::sync::Arc;

use jellyfin::runtime::JellyfinRuntime;
use zayden_core::{InvocationCtx, parse_options, parse_subcommand};

use crate::error::{Result, WatchError};
use crate::party::Scheduled;

pub async fn run(
    cx: &InvocationCtx<'_>,
    runtime: &Arc<JellyfinRuntime>,
) -> Result<Scheduled> {
    let (_, group_options) = parse_subcommand(cx.interaction.data.options())?;
    let (name, leaf_options) = parse_subcommand(group_options)?;
    let options = parse_options(leaf_options);

    match name {
        "create" => create::run(cx, runtime, options).await,
        "list" => list::run(cx, runtime).await.map(|()| Scheduled::Nothing),
        "info" => info::run(cx, runtime, options).await.map(|()| Scheduled::Nothing),
        "cancel" => cancel::run(cx, runtime, options).await,
        _ => Err(WatchError::UnknownSubcommand(name.to_string())),
    }
}
