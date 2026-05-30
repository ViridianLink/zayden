use std::sync::Arc;

use serenity::all::{Context, ModalInteraction};
use tracing::{info, warn};
use zayden_app::state::AppState;
use zayden_core::parse_modal_components;

use super::respond_with_error;
use crate::{CommandRegistry, Handler, Result};

impl Handler {
    pub async fn interaction_modal(
        ctx: &Context,
        interaction: &ModalInteraction,
        app: Arc<AppState>,
        registry: Arc<CommandRegistry>,
    ) -> Result<()> {
        let inputs = parse_modal_components(interaction.data.components.as_slice());

        info!(
            "{} ran modal: {} {:?}",
            interaction.user.name, interaction.data.custom_id, inputs
        );

        match registry.run_modal(ctx, interaction, app).await {
            Some(Ok(())) => {},
            Some(Err(e)) => respond_with_error(ctx, interaction, e).await,
            None => warn!(
                custom_id = interaction.data.custom_id.as_str(),
                user = interaction.user.name.as_str(),
                "modal not implemented",
            ),
        }

        Ok(())
    }
}
