use std::sync::Arc;

use serenity::all::{
    ComponentInteraction, Context, CreateInteractionResponse, EditInteractionResponse,
};
use sqlx::PgPool;
use tracing::{error, info};
use zayden_app::state::AppState;
use zayden_core::error::{HandlerError, Respond};

use crate::handler::Handler;
use crate::{CommandRegistry, Error, Result};

impl Handler {
    pub async fn interaction_component(
        ctx: &Context,
        interaction: &ComponentInteraction,
        _pool: &PgPool,
        app: Arc<AppState>,
        registry: Arc<CommandRegistry>,
    ) -> Result<()> {
        let custom_id = &interaction.data.custom_id;

        info!(
            "{} ran component: {} - {}",
            interaction.user.name, custom_id, interaction.message.id,
        );

        if let Some(result) = registry.run_component(ctx, interaction, app).await {
            if let Err(err) = result {
                report_handler_error(ctx, interaction, err).await;
            }
            return Ok(());
        }

        let result: Result<()> = interaction
            .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
            .await
            .map_err(Error::from);

        if let Err(err) = result {
            report(ctx, interaction, err).await;
        }

        Ok(())
    }
}

async fn report_handler_error(
    ctx: &Context,
    interaction: &ComponentInteraction,
    err: HandlerError,
) {
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
                message_id = %interaction.message.id,
                "internal error in component handler",
            );
        }
    }
}

async fn report(ctx: &Context, interaction: &ComponentInteraction, err: Error) {
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
                message_id = %interaction.message.id,
                "internal error in component handler",
            );
        }
    }
}
