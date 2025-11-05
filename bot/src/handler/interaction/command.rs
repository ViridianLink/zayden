use serenity::all::{CommandInteraction, Context, EditInteractionResponse};
use sqlx::PgPool;
use tracing::{debug, info, warn};
use zayden_core::get_option_str;

use crate::Result;
use crate::handler::Handler;
use crate::modules::APPLICATION_COMMANDS;

impl Handler {
    pub async fn interaction_command(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let options = interaction.data.options();

        info!(
            "{} ran command: {}{}",
            interaction.user.name,
            interaction.data.name,
            get_option_str(&options)
        );

        let result = match APPLICATION_COMMANDS.get(interaction.data.name.as_str()) {
            Some(cmd) => cmd.run(ctx, interaction, options, pool),
            None => {
                warn!("Unknown command: {}", interaction.data.name);
                return Ok(());
            }
        }
        .await;

        if let Err(e) = result.as_ref() {
            let msg = e.to_string();

            if !msg.is_empty() {
                debug!("Sent error: {e:?}\n{interaction:?}");

                let _ = interaction.defer_ephemeral(&ctx.http).await;

                interaction
                    .edit_response(&ctx.http, EditInteractionResponse::new().content(msg))
                    .await
                    .unwrap();

                return Ok(());
            }
        }

        result
    }
}
