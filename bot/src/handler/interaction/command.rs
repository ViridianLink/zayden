use serenity::all::{CommandInteraction, Context, EditInteractionResponse};
use sqlx::PgPool;
use tracing::{info, warn};
use zayden_core::error::Respond;
use zayden_core::get_option_str;

use crate::Error;
use crate::handler::Handler;
use crate::modules::APPLICATION_COMMANDS;

impl Handler {
    pub async fn interaction_command(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) {
        let options = interaction.data.options();

        info!(
            "{} ran command: {}{}",
            interaction.user.name,
            interaction.data.name,
            get_option_str(&options)
        );

        let result = match APPLICATION_COMMANDS.get(interaction.data.name.as_str()) {
            Some(cmd) => cmd.run(ctx, interaction, options, pool).await,
            None => {
                warn!("Unknown command: {}", interaction.data.name);
                Ok(())
            }
        };

        if let Err(e) = result {
            report(ctx, interaction, e).await;
        }
    }
}

async fn report(ctx: &Context, interaction: &CommandInteraction, err: Error) {
    let cmd = interaction.data.name.as_str();
    let user = interaction.user.name.as_str();

    match err.user_message() {
        Some(msg) => {
            info!(error = ?err, command = cmd, user = user, "user error");

            let _ = interaction.defer_ephemeral(&ctx.http).await;
            if let Err(send_err) = interaction
                .edit_response(&ctx.http, EditInteractionResponse::new().content(msg))
                .await
            {
                warn!(error = ?err, send_err = ?send_err, "failed to deliver user error");
            }
        }
        None => {
            tracing::error!(error = ?err, command=cmd, user=user, interaction=?interaction, "internal error");
        }
    }
}
