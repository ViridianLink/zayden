use std::borrow::Cow;

use futures::{StreamExt, TryStreamExt};
use serenity::all::{
    ComponentInteraction, ComponentInteractionDataKind, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateModal, CreateModalComponent, EditThread, Http,
};
use sqlx::{Database, Pool};

use crate::{Error, Result, TicketGuildManager};

pub struct TicketComponent;

impl TicketComponent {
    pub async fn ticket_create<'a>(
        http: &Http,
        interaction: &ComponentInteraction,
        components: impl Into<Cow<'a, [CreateModalComponent<'a>]>>,
    ) -> Result<()> {
        let modal = CreateModal::new("create_ticket", "Ticket").components(components);

        interaction
            .create_response(http, CreateInteractionResponse::Modal(modal))
            .await?;

        Ok(())
    }

    pub async fn support_close(http: &Http, interaction: &ComponentInteraction) -> Result<()> {
        let channel = interaction.channel.as_ref().unwrap();

        let new_channel_name: String =
            format!("{} - {}", "[Closed]", channel.base().name.as_ref().unwrap())
                .chars()
                .take(100)
                .collect();

        channel
            .id()
            .expect_thread()
            .edit(
                http,
                EditThread::new().name(new_channel_name).archived(true),
            )
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
        let guild_id = interaction.guild_id.ok_or(Error::MissingGuildId)?;

        let index = match &interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => {
                values[0].parse::<usize>().unwrap()
            }
            _ => unreachable!("Invalid interaction data kind"),
        };

        let faq_channel_id = GuildManager::get(pool, guild_id)
            .await
            .unwrap()
            .unwrap()
            .faq_channel_id()
            .unwrap();

        let message = faq_channel_id
            .widen()
            .messages_iter(http)
            .skip(index)
            .boxed()
            .try_next()
            .await
            .unwrap()
            .unwrap();

        let mut parts: Vec<&str> = message.content.split("**").collect();
        let description = parts.pop().unwrap().trim();
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
