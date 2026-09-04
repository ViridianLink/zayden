use std::borrow::Cow;
use std::sync::Arc;

use serenity::all::{
    ComponentInteraction,
    ComponentInteractionDataKind,
    CreateInteractionResponse,
    CreateInteractionResponseFollowup,
    CreateInteractionResponseMessage,
    CreateModal,
    CreateModalComponent,
    EditInteractionResponse,
    EditThread,
    GenericInteractionChannel,
    Http,
    InteractionGuildThread,
    MessageFlags,
    Permissions,
};
use zayden_app::state::AppState;
use zayden_core::{CoreError as ZaydenError, as_i64};

use crate::faq::{FaqArticle, views};
use crate::idle::{ThreadActivity, may_act};
use crate::{
    Result,
    TicketError,
    TicketGuildRow,
    TicketStores,
    donation,
    solve,
    state,
};

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

        ThreadActivity::pause(&app.db, thread.id).await?;

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

    pub async fn support_solved(
        http: &Arc<Http>,
        interaction: &ComponentInteraction,
        app: &Arc<AppState>,
    ) -> Result<()> {
        let stores = TicketStores::from_app(app);
        let pool = &app.db;

        let (thread, row, _activity) = Self::nudge_target(interaction, app).await?;
        let support_channel_id =
            row.channel_id().ok_or(TicketError::NotInSupportChannel)?;
        let guild_id = interaction.guild_id.ok_or(ZaydenError::MissingGuildId)?;

        interaction
            .create_response(http, CreateInteractionResponse::Acknowledge)
            .await?;

        solve::mark_solved(
            http,
            app,
            stores,
            guild_id,
            &row,
            support_channel_id,
            thread,
        )
        .await?;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new()
                    .content(format!(
                        "{} marked this solved. Thanks!",
                        interaction.user.display_name()
                    ))
                    .components(Vec::new()),
            )
            .await?;

        if let Some(message) =
            donation::message(http, pool, thread.id, guild_id, row.role_ids())
                .await?
        {
            interaction
                .create_followup(
                    http,
                    CreateInteractionResponseFollowup::new().content(message),
                )
                .await?;
        }

        Ok(())
    }

    pub async fn support_still_open(
        http: &Http,
        interaction: &ComponentInteraction,
        app: &AppState,
    ) -> Result<()> {
        let (thread, _row, _activity) = Self::nudge_target(interaction, app).await?;

        interaction
            .create_response(http, CreateInteractionResponse::Acknowledge)
            .await?;

        ThreadActivity::resume(&app.db, thread.id).await?;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new()
                    .content("Thanks - this is back in the support team's queue.")
                    .components(Vec::new()),
            )
            .await?;

        Ok(())
    }

    async fn nudge_target<'a>(
        interaction: &'a ComponentInteraction,
        app: &AppState,
    ) -> Result<(&'a InteractionGuildThread, TicketGuildRow, ThreadActivity)> {
        let guild_id = interaction.guild_id.ok_or(ZaydenError::MissingGuildId)?;

        let GenericInteractionChannel::Thread(thread) = &interaction.channel else {
            return Err(TicketError::NotInSupportChannel);
        };

        let activity = ThreadActivity::active(&app.db, thread.id)
            .await?
            .ok_or(TicketError::TicketAlreadyClosed)?;

        let row =
            TicketGuildRow::get(TicketStores::from_app(app), &app.db, guild_id)
                .await?
                .ok_or(TicketError::NotInSupportChannel)?;

        let (roles, manage) = interaction.member.as_ref().map_or_else(
            || (Vec::new(), false),
            |member| {
                (
                    member.roles.to_vec(),
                    member.permissions.is_some_and(|permissions| {
                        permissions.contains(Permissions::MANAGE_MESSAGES)
                    }),
                )
            },
        );

        if !may_act(
            interaction.user.id,
            activity.op(),
            &roles,
            row.role_ids(),
            manage,
        ) {
            return Err(TicketError::NotTicketParticipant);
        }

        Ok((thread, row, activity))
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
