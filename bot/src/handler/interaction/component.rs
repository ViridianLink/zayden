use std::sync::Arc;

use serenity::all::{ComponentInteraction, Context};
use tracing::{info, warn};
use zayden_app::state::AppState;

use super::respond_with_error;
use crate::handler::Handler;
use crate::{CommandRegistry, Result};

impl Handler {
    pub async fn interaction_component(
        ctx: &Context,
        interaction: &ComponentInteraction,
        app: Arc<AppState>,
        registry: Arc<CommandRegistry>,
    ) -> Result<()> {
        let custom_id = &interaction.data.custom_id;

        info!(
            "{} ran component: {} - {}",
            interaction.user.name, custom_id, interaction.message.id,
        );

        match registry.run_component(ctx, interaction, app).await {
            Some(Ok(())) => {},
            Some(Err(e)) => respond_with_error(ctx, interaction, e).await,
            None => warn!(custom_id = custom_id.as_str(), "unknown component"),
        }

        Ok(())
    }
}
