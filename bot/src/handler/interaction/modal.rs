use chrono::Utc;
use serenity::all::{Context, EditInteractionResponse, ModalInteraction};
use sqlx::{PgPool, Postgres};
use suggestions::Suggestions;
use ticket::TicketModal;

use crate::handler::Handler;
use crate::modules::lfg::{PostTable, UsersTable};
use crate::modules::ticket::TicketTable;
use crate::sqlx_lib::GuildTable;
use crate::{Error, Result, modules};

impl Handler {
    pub async fn interaction_modal(
        ctx: &Context,
        interaction: &ModalInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        println!(
            "[{}] {} ran modal: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            interaction.user.name,
            interaction.data.custom_id
        );

        let result = match interaction.data.custom_id.as_str() {
            // region LFG
            "lfg_edit" => {
                lfg::modals::Edit::run::<Postgres, PostTable, UsersTable>(ctx, interaction, pool)
                    .await
                    .map_err(Error::from)
            }
            custom_id if custom_id.starts_with("lfg_create") => lfg::modals::Create::run::<
                Postgres,
                modules::lfg::GuildTable,
                PostTable,
                UsersTable,
            >(
                ctx, interaction, pool
            )
            .await
            .map_err(Error::from),
            // endregion

            // region Ticket
            "create_ticket" => {
                TicketModal::run::<Postgres, GuildTable, TicketTable>(ctx, interaction, pool)
                    .await
                    .map_err(Error::from)
            }
            // endregion
            "suggestions_accept" => {
                Suggestions::modal(ctx, interaction, true).await;
                Ok(())
            }
            "suggestions_reject" => {
                Suggestions::modal(ctx, interaction, false).await;
                Ok(())
            }

            _ => unimplemented!("Modal not implemented: {}", interaction.data.custom_id),
        };

        if let Err(e) = result {
            let msg = e.to_string();

            let _ = interaction.defer_ephemeral(ctx).await;

            interaction
                .edit_response(ctx, EditInteractionResponse::new().content(msg))
                .await
                .unwrap();
        }

        Ok(())
    }
}
