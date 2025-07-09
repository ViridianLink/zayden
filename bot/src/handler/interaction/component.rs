use chrono::Utc;
use serenity::all::{ComponentInteraction, Context, EditInteractionResponse};
use sqlx::{PgPool, Postgres};
use suggestions::Suggestions;
use zayden_core::Component;

use crate::handler::Handler;
use crate::modules::gambling::Blackjack;
use crate::modules::lfg::PostTable;
use crate::modules::ticket::Ticket;
use crate::{Error, Result};

impl Handler {
    pub async fn interaction_component(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        println!(
            "[{}] {} ran component: {} - {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S"),
            interaction.user.name,
            interaction.data.custom_id,
            interaction.message.id,
        );

        let result = match interaction.data.custom_id.as_str() {
            //region: Gambling
            id if id.starts_with("blackjack") => Blackjack::run(ctx, interaction, pool).await,

            // region: Lfg
            "lfg_join" => lfg::Components::join::<Postgres, PostTable>(ctx, interaction, pool)
                .await
                .map_err(Error::from),
            "lfg_leave" => lfg::Components::leave::<Postgres, PostTable>(ctx, interaction, pool)
                .await
                .map_err(Error::from),
            "lfg_alternative" => {
                lfg::Components::alternative::<Postgres, PostTable>(ctx, interaction, pool)
                    .await
                    .map_err(Error::from)
            }
            "lfg_settings" => {
                lfg::Components::settings::<Postgres, PostTable>(ctx, interaction, pool)
                    .await
                    .map_err(Error::from)
            }

            "lfg_edit" => lfg::Components::edit::<Postgres, PostTable>(ctx, interaction, pool)
                .await
                .map_err(Error::from),
            "lfg_copy" => lfg::Components::copy::<Postgres, PostTable>(ctx, interaction, pool)
                .await
                .map_err(Error::from),
            "lfg_kick" => lfg::Components::kick::<Postgres, PostTable>(ctx, interaction, pool)
                .await
                .map_err(Error::from),
            "lfg_kick_menu" => {
                lfg::KickComponent::run::<Postgres, PostTable>(ctx, interaction, pool)
                    .await
                    .map_err(Error::from)
            }
            "lfg_delete" => lfg::Components::delete::<Postgres, PostTable>(ctx, interaction, pool)
                .await
                .map_err(Error::from),
            // "lfg_tags_add" => Lfg::tags_add(ctx, interaction).await,
            // "lfg_tags_remove" => Lfg::tags_remove(ctx, interaction).await,
            // endregion
            "suggestions_accept" | "suggestions_added" | "accept" => {
                Suggestions::components(ctx, interaction, true).await;
                Ok(())
            }
            "suggestions_reject" | "reject" => {
                Suggestions::components(ctx, interaction, false).await;
                Ok(())
            }

            //region: Ticket
            "ticket_create" | "support_ticket" => Ticket::ticket_create(ctx, interaction).await,
            "support_close" => Ticket::support_close(ctx, interaction).await,
            "support_faq" => Ticket::support_faq(ctx, interaction, pool).await,
            //endregion: Ticket
            _ => Ok(()),
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
