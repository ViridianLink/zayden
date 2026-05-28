use std::sync::Arc;

use serenity::all::{CommandInteraction, Context};
use sqlx::PgPool;
use tracing::{error, warn};
use zayden_app::state::AppState;

use crate::CommandRegistry;
use crate::Result;
use crate::handler::Handler;

impl Handler {
    pub async fn interaction_autocomplete(
        ctx: &Context,
        interaction: &CommandInteraction,
        _pool: &PgPool,
        app: Arc<AppState>,
        registry: Arc<CommandRegistry>,
    ) -> Result<()> {
        if let Some(result) = registry.run_autocomplete(ctx, interaction, app).await {
            if let Err(err) = result {
                error!(
                    error = ?err,
                    command = interaction.data.name.as_str(),
                    user = interaction.user.name.as_str(),
                    "autocomplete handler error",
                );
            }
            return Ok(());
        }

        warn!("Unknown autocomplete command: {}", interaction.data.name);
        Ok(())
    }
}
