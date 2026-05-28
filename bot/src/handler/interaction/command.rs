use std::sync::Arc;

use serenity::all::{CommandInteraction, Context, EditInteractionResponse};
use sqlx::PgPool;
use tracing::{error, info, warn};
use zayden_app::state::AppState;
use zayden_core::error::{HandlerError, Respond};
use zayden_core::get_option_str;

use crate::bindings::APPLICATION_COMMANDS;
use crate::handler::Handler;
use crate::{CommandRegistry, Error};

impl Handler {
    pub async fn interaction_command(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
        app: Arc<AppState>,
        registry: Arc<CommandRegistry>,
    ) {
        let options = interaction.data.options();

        info!(
            "{} ran command: {}{}",
            interaction.user.name,
            interaction.data.name,
            get_option_str(&options)
        );

        if let Some(result) = registry.run_command(ctx, interaction, app).await {
            if let Err(e) = result {
                report_handler_error(ctx, interaction, e).await;
            }
            return;
        }

        let result = match APPLICATION_COMMANDS.get(interaction.data.name.as_str()) {
            Some(cmd) => cmd.run(ctx, interaction, options, pool).await,
            None => {
                warn!(command = interaction.data.name.as_str(), "unknown command");
                Ok(())
            }
        };

        if let Err(e) = result {
            report(ctx, interaction, e).await;
        }
    }
}

async fn report_handler_error(ctx: &Context, interaction: &CommandInteraction, err: HandlerError) {
    let cmd = interaction.data.name.as_str();
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
                    command = cmd,
                    user = user,
                    "failed to deliver user error message",
                );
            }
        }
        None => {
            error!(
                error = ?err,
                command = cmd,
                user = user,
                channel_id = %interaction.channel_id,
                guild_id = ?interaction.guild_id,
                "internal error in command handler",
            );
        }
    }
}

async fn report(ctx: &Context, interaction: &CommandInteraction, err: Error) {
    let cmd = interaction.data.name.as_str();
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
                    command = cmd,
                    user = user,
                    "failed to deliver user error message",
                );
            }
        }
        None => {
            error!(
                error = ?err,
                command = cmd,
                user = user,
                channel_id = %interaction.channel_id,
                guild_id = ?interaction.guild_id,
                "internal error in command handler",
            );
        }
    }
}
