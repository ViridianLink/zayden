use std::sync::Arc;

use serenity::all::{Context, Interaction};
use sqlx::PgPool;
use tracing::warn;
use zayden_app::state::AppState;

mod autocomplete;
mod command;
mod component;
mod modal;

use crate::{CommandRegistry, Result};

use super::Handler;

impl Handler {
    pub async fn interaction_create(
        ctx: &Context,
        interaction: &Interaction,
        pool: &PgPool,
        app: Arc<AppState>,
        registry: Arc<CommandRegistry>,
    ) -> Result<()> {
        match interaction {
            Interaction::Command(command) => {
                Handler::interaction_command(
                    ctx,
                    command,
                    pool,
                    Arc::clone(&app),
                    Arc::clone(&registry),
                )
                .await;
                Ok(())
            }
            Interaction::Autocomplete(autocomplete) => {
                Handler::interaction_autocomplete(ctx, autocomplete, pool).await
            }
            Interaction::Component(component) => {
                Handler::interaction_component(
                    ctx,
                    component,
                    pool,
                    Arc::clone(&app),
                    Arc::clone(&registry),
                )
                .await
            }
            Interaction::Modal(modal) => Handler::interaction_modal(ctx, modal, pool).await,
            other => {
                warn!(kind = ?other.kind(), "interaction kind not handled");
                Ok(())
            }
        }
    }
}
