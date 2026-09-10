mod components;
mod slash_commands;

use std::sync::Arc;

use hosting::{HostingError, HostingRuntime};
use serenity::all::Context;
use tokio::sync::RwLock;
use zayden_core::error::HandlerError;

pub use crate::bindings::hosting::components::{
    HostingCancel,
    HostingConfirm,
    HostingSpec,
};
pub use crate::bindings::hosting::slash_commands::Server;
use crate::registry::OverlapError;
use crate::{BotState, RegistryBuilder};

pub(crate) async fn runtime(
    ctx: &Context,
) -> Result<Arc<HostingRuntime>, HandlerError> {
    let data = ctx.data::<RwLock<BotState>>();
    let guard = data.read().await;
    let found = guard.hosting.as_ref().map(Arc::clone);
    drop(guard);

    found.ok_or_else(|| HandlerError::from_respond(HostingError::Disabled))
}

pub fn register(builder: &mut RegistryBuilder) -> Result<(), OverlapError> {
    builder
        .add_command(Server)
        .add_autocomplete(Server)
        .add_modal(HostingSpec)?
        .add_component(HostingConfirm)?
        .add_component(HostingCancel)?;

    Ok(())
}
