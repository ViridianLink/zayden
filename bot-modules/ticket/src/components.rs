use std::borrow::Cow;

use futures::{StreamExt, TryStreamExt};
use serenity::all::{
    ComponentInteraction,
    ComponentInteractionDataKind,
    CreateEmbed,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateModal,
    CreateModalComponent,
    EditThread,
    Http,
};
use sqlx::PgPool;
use zayden_core::CoreError as ZaydenError;

use crate::{Result, TicketError, TicketGuildRow, TicketStores};

pub struct TicketComponent;

impl TicketComponent {
    pub async fn ticket_create<'a>(
        http: &Http,
        interaction: &ComponentInteraction,
        components: impl Into<Cow<'a, [CreateModalComponent<'a>]>>,
    ) -> Result<()> {
        let modal =
            CreateModal::new("create_ticket", "Ticket").components(components);

        interaction
            .create_response(http, CreateInteractionResponse::Modal(modal))
            .await?;

        Ok(())
    }

    pub async fn support_close(
        http: &Http,
        interaction: &ComponentInteraction,
    ) -> Result<()> {
        let Some(channel) = interaction.channel.as_ref() else {
            return Err(TicketError::Internal(
                "TicketComponent::support_close: interaction has no associated channel"
                    .into(),
            ));
        };

        let new_channel_name: String = format!(
            "[Closed] - {}",
            channel.base().name.as_deref().unwrap_or_default()
        )
        .chars()
        .take(100)
        .collect();

        channel
            .id()
            .expect_thread()
            .edit(http, EditThread::new().name(new_channel_name).archived(true))
            .await?;

        interaction
            .create_response(http, CreateInteractionResponse::Acknowledge)
            .await?;

        Ok(())
    }

    pub async fn support_faq(
        http: &Http,
        interaction: &ComponentInteraction,
        stores: TicketStores<'_>,
        pool: &PgPool,
    ) -> Result<()> {
        let guild_id = interaction.guild_id.ok_or(ZaydenError::MissingGuildId)?;

        let ComponentInteractionDataKind::StringSelect { values } =
            &interaction.data.kind
        else {
            return Err(TicketError::Internal(
                "TicketComponent::support_faq: expected StringSelect interaction"
                    .into(),
            ));
        };

        let Some(raw) = values.first() else {
            return Err(TicketError::Internal(
                "TicketComponent::support_faq: StringSelect had no values".into(),
            ));
        };

        let index =
            raw.parse::<usize>().map_err(|_e| TicketError::SupportNotFound)?;

        let faq_channel_id = TicketGuildRow::get(stores, pool, guild_id)
            .await?
            .ok_or(ZaydenError::MissingGuildId)?
            .faq_channel_id()
            .ok_or(ZaydenError::MissingGuildId)?;

        let message = faq_channel_id
            .widen()
            .messages_iter(http)
            .skip(index)
            .boxed()
            .try_next()
            .await?
            .ok_or(TicketError::SupportNotFound)?;

        let mut parts: Vec<&str> = message.content.split("**").collect();
        let description = parts.pop().unwrap_or_default().trim();
        let title = parts.join("");

        interaction
            .create_response(
                http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().embed(
                        CreateEmbed::new()
                            .title(title.trim())
                            .description(description),
                    ),
                ),
            )
            .await?;

        Ok(())
    }
}
