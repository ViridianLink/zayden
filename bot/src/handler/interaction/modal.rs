use serenity::all::{Context, EditInteractionResponse, ModalInteraction};
use sqlx::{PgPool, Postgres};
use suggestions::Suggestions;
use ticket::TicketModal;
use tracing::{error, info, warn};
use zayden_core::error::Respond;
use zayden_core::parse_modal_components;

use crate::bindings::lfg::{PostTable, UsersTable};
use crate::bindings::ticket::TicketTable;
use crate::sqlx_lib::GuildTable;
use crate::{BotState, Error, Handler, Result};

impl Handler {
    pub async fn interaction_modal(
        ctx: &Context,
        interaction: &ModalInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let inputs = parse_modal_components(interaction.data.components.as_slice());

        info!(
            "{} ran modal: {} {:?}",
            interaction.user.name, interaction.data.custom_id, inputs
        );

        let result: Result<()> = match interaction.data.custom_id.as_str() {
            // region LFG
            "lfg_edit" => lfg::modals::Edit::run::<BotState, Postgres, PostTable, UsersTable>(
                ctx,
                interaction,
                pool,
            )
            .await
            .map_err(Error::from),
            custom_id if custom_id.starts_with("lfg_create") => {
                lfg::modals::Create::run::<BotState, Postgres, GuildTable, PostTable, UsersTable>(
                    ctx,
                    interaction,
                    pool,
                )
                .await
                .map_err(Error::from)
            }
            // endregion

            // region Ticket
            "create_ticket" => {
                TicketModal::run::<Postgres, GuildTable, TicketTable>(&ctx.http, interaction, pool)
                    .await
                    .map_err(Error::from)
            }
            // endregion
            "suggestions_accept" => Suggestions::modal(&ctx.http, interaction, true)
                .await
                .map_err(Error::from),
            "suggestions_reject" => Suggestions::modal(&ctx.http, interaction, false)
                .await
                .map_err(Error::from),

            unknown => {
                warn!(
                    custom_id = unknown,
                    user = interaction.user.name.as_str(),
                    "modal not implemented",
                );
                Ok(())
            }
        };

        if let Err(err) = result {
            report(ctx, interaction, err).await;
        }

        Ok(())
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
