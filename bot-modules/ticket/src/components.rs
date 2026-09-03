use std::borrow::Cow;

use serenity::all::{
    ComponentInteraction,
    ComponentInteractionDataKind,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateModal,
    CreateModalComponent,
    EditThread,
    Http,
};
use zayden_app::state::AppState;
use zayden_core::{CoreError as ZaydenError, as_i64};

use crate::faq::{FaqArticle, embeds};
use crate::{Result, TicketError};

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
        let channel = &interaction.channel;

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
        app: &AppState,
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

        let id = raw.parse::<i32>().map_err(|_e| TicketError::ArticleNotFound)?;

        let article = FaqArticle::get(&app.db, as_i64(guild_id.get()), id)
            .await?
            .ok_or(TicketError::ArticleNotFound)?;

        interaction
            .create_response(
                http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .embed(embeds::stored(&article)),
                ),
            )
            .await?;

        Ok(())
    }
}
