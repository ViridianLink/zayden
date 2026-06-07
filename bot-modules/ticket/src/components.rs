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
use sqlx::{Database, Pool};
use zayden_core::Error as ZaydenError;

use crate::{Error, Result, TicketGuildManager};

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
            return Ok(());
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

    pub async fn support_faq<Db: Database, GuildManager: TicketGuildManager<Db>>(
        http: &Http,
        interaction: &ComponentInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let guild_id = interaction.guild_id.ok_or(ZaydenError::MissingGuildId)?;

        let ComponentInteractionDataKind::StringSelect { values } =
            &interaction.data.kind
        else {
            return Ok(());
        };

        let Some(raw) = values.first() else {
            return Ok(());
        };

        let index = raw.parse::<usize>().map_err(|_e| Error::SupportNotFound)?;

        let faq_channel_id = GuildManager::get(pool, guild_id)
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
            .ok_or(Error::SupportNotFound)?;

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
