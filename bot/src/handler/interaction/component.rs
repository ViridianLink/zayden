use chrono::Utc;
use serenity::all::{
    ComponentInteraction, Context, CreateInteractionResponse, EditInteractionResponse,
};
use sqlx::{PgPool, Postgres};
use suggestions::Suggestions;
use ticket::TicketComponent;
use zayden_core::Component;

use crate::handler::Handler;
use crate::modules::gambling::{Blackjack, HigherLower, Prestige};
use crate::modules::lfg::PostTable;
use crate::modules::ticket::Ticket;
use crate::sqlx_lib::GuildTable;
use crate::{Error, Result};

impl Handler {
    pub async fn interaction_component(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let custom_id = &interaction.data.custom_id;

        println!(
            "[{}] {} ran component: {} - {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            interaction.user.name,
            custom_id,
            interaction.message.id,
        );

        let result = match custom_id.as_str() {
            //region: Gambling
            id if id.starts_with("blackjack") => Blackjack::run(ctx, interaction, pool).await,
            id if id.starts_with("hol") => HigherLower::run(ctx, interaction, pool).await,
            id if id.starts_with("prestige") => Prestige::run(ctx, interaction, pool).await,
            //endregion

            // region: Lfg
            "lfg_join" => {
                lfg::Components::join::<Postgres, PostTable>(&ctx.http, interaction, pool)
                    .await
                    .map_err(Error::from)
            }
            "lfg_leave" => {
                lfg::Components::leave::<Postgres, PostTable>(&ctx.http, interaction, pool)
                    .await
                    .map_err(Error::from)
            }
            "lfg_alternative" => {
                lfg::Components::alternative::<Postgres, PostTable>(&ctx.http, interaction, pool)
                    .await
                    .map_err(Error::from)
            }
            "lfg_settings" => {
                lfg::Components::settings::<Postgres, PostTable>(&ctx.http, interaction, pool)
                    .await
                    .map_err(Error::from)
            }

            "lfg_edit" => {
                lfg::Components::edit::<Postgres, PostTable>(&ctx.http, interaction, pool)
                    .await
                    .map_err(Error::from)
            }
            "lfg_copy" => {
                lfg::Components::copy::<Postgres, PostTable>(&ctx.http, interaction, pool)
                    .await
                    .map_err(Error::from)
            }
            "lfg_kick" => {
                lfg::Components::kick::<Postgres, PostTable>(&ctx.http, interaction, pool)
                    .await
                    .map_err(Error::from)
            }
            "lfg_kick_menu" => {
                lfg::KickComponent::run::<Postgres, PostTable>(&ctx.http, interaction, pool)
                    .await
                    .map_err(Error::from)
            }
            "lfg_delete" => {
                lfg::Components::delete::<Postgres, PostTable>(&ctx.http, interaction, pool)
                    .await
                    .map_err(Error::from)
            }
            // "lfg_tags_add" => Lfg::tags_add(ctx, interaction).await,
            // "lfg_tags_remove" => Lfg::tags_remove(ctx, interaction).await,
            // endregion
            "suggestions_accept" | "suggestions_added" | "accept" => {
                Suggestions::components(&ctx.http, interaction, true).await;
                Ok(())
            }
            "suggestions_reject" | "reject" => {
                Suggestions::components(&ctx.http, interaction, false).await;
                Ok(())
            }

            //region: Ticket
            "ticket_create" | "support_ticket" => {
                Ticket::ticket_create(&ctx.http, interaction).await
            }
            "support_close" => TicketComponent::support_close(&ctx.http, interaction)
                .await
                .map_err(Error::from),
            "support_faq" => {
                TicketComponent::support_faq::<Postgres, GuildTable>(&ctx.http, interaction, pool)
                    .await
                    .map_err(Error::from)
            }
            //endregion: Ticket
            _ => interaction
                .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
                .await
                .map_err(Error::from),
        };

        if let Err(e) = result {
            let msg = e.to_string();

            let _ = interaction.defer_ephemeral(&ctx.http).await;

            interaction
                .edit_response(&ctx.http, EditInteractionResponse::new().content(msg))
                .await
                .unwrap();
        }

        Ok(())
    }
}
