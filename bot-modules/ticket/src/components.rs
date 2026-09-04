use std::borrow::Cow;

use serenity::all::{
    ComponentInteraction,
    ComponentInteractionDataKind,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateModal,
    CreateModalComponent,
    EditThread,
    GenericInteractionChannel,
    Http,
    MessageFlags,
};
use zayden_app::state::AppState;
use zayden_core::{CoreError as ZaydenError, as_i64};

use crate::faq::{FaqArticle, views};
use crate::{Result, TicketError, TicketGuildRow, TicketStores, state};

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
        app: &AppState,
    ) -> Result<()> {
        let guild_id = interaction.guild_id.ok_or(ZaydenError::MissingGuildId)?;
        // The button only ever rides on a ticket's opening message, but the id
        // cast below would rename a whole channel if it were pressed elsewhere.
        let GenericInteractionChannel::Thread(thread) = &interaction.channel else {
            return Err(TicketError::NotInSupportChannel);
        };

        let row =
            TicketGuildRow::get(TicketStores::from_app(app), &app.db, guild_id)
                .await?
                .ok_or(TicketError::NotInSupportChannel)?;
        let support_channel_id =
            row.channel_id().ok_or(TicketError::NotInSupportChannel)?;

        state::mark(
            http,
            guild_id,
            support_channel_id,
            thread,
            row.closed_tag_id(),
            state::CLOSED,
        )
        .await?;

        thread.id.edit(http, EditThread::new().archived(true)).await?;

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
                        .components(vec![views::stored(&article)])
                        .flags(MessageFlags::IS_COMPONENTS_V2),
                ),
            )
            .await?;

        Ok(())
    }
}
