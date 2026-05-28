use std::sync::Arc;

use serenity::all::{Context, EditInteractionResponse, ModalInteraction};
use tracing::{error, info, warn};
use zayden_app::state::AppState;
use zayden_core::error::{HandlerError, Respond};
use zayden_core::parse_modal_components;

use crate::{CommandRegistry, Error, Handler, Result};

impl Handler {
    pub async fn interaction_modal(
        ctx: &Context,
        interaction: &ModalInteraction,
        _pool: &sqlx::PgPool,
        app: Arc<AppState>,
        registry: Arc<CommandRegistry>,
    ) -> Result<()> {
        let inputs = parse_modal_components(interaction.data.components.as_slice());

        info!(
            "{} ran modal: {} {:?}",
            interaction.user.name, interaction.data.custom_id, inputs
        );

        if let Some(result) = registry.run_modal(ctx, interaction, app).await {
            if let Err(err) = result {
                report_handler_error(ctx, interaction, err).await;
            }
            return Ok(());
        }

        warn!(
            custom_id = interaction.data.custom_id.as_str(),
            user = interaction.user.name.as_str(),
            "modal not implemented",
        );
        let result: Result<()> = Ok(());

        if let Err(err) = result {
            report(ctx, interaction, err).await;
        }

        Ok(())
    }
}

async fn report_handler_error(ctx: &Context, interaction: &ModalInteraction, err: HandlerError) {
    let custom_id = interaction.data.custom_id.as_str();
    let user = interaction.user.name.as_str();

    match err.user_message() {
        Some(msg) => {
            let _ = interaction.defer_ephemeral(&ctx.http).await;
            if let Err(send_err) = interaction
                .edit_response(&ctx.http, EditInteractionResponse::new().content(msg))
                .await
            {
                error!(
                    error = ?err,
                    send_err = ?send_err,
                    custom_id = custom_id,
                    user = user,
                    "failed to deliver user error message",
                );
            }
        }
        None => {
            error!(
                error = ?err,
                custom_id = custom_id,
                user = user,
                channel_id = %interaction.channel_id,
                guild_id = ?interaction.guild_id,
                "internal error in modal handler",
            );
        }
    }
}

async fn report(ctx: &Context, interaction: &ModalInteraction, err: Error) {
    let custom_id = interaction.data.custom_id.as_str();
    let user = interaction.user.name.as_str();

    match err.user_message() {
        Some(msg) => {
            let _ = interaction.defer_ephemeral(&ctx.http).await;
            if let Err(send_err) = interaction
                .edit_response(&ctx.http, EditInteractionResponse::new().content(msg))
                .await
            {
                error!(
                    error = ?err,
                    send_err = ?send_err,
                    custom_id = custom_id,
                    user = user,
                    "failed to deliver user error message",
                );
            }
        }
        None => {
            error!(
                error = ?err,
                custom_id = custom_id,
                user = user,
                channel_id = %interaction.channel_id,
                guild_id = ?interaction.guild_id,
                "internal error in modal handler",
            );
        }
    }
}
