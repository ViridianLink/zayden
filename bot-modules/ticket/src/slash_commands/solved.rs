use std::sync::Arc;

use serenity::all::{
    CommandInteraction,
    CreateInteractionResponseFollowup,
    EditInteractionResponse,
    GuildId,
    Http,
    Permissions,
};
use zayden_app::state::AppState;

use crate::archive::notice;
use crate::idle::{ThreadActivity, may_act};
use crate::{
    Result,
    Ticket,
    TicketError,
    TicketGuildRow,
    TicketStores,
    donation,
    solve,
    support_thread,
};

impl Ticket {
    pub(super) async fn solved(
        http: &Arc<Http>,
        interaction: &CommandInteraction,
        stores: TicketStores<'_>,
        app: &Arc<AppState>,
        guild_id: GuildId,
    ) -> Result<()> {
        let pool = &app.db;

        interaction.defer(http).await?;

        let row = TicketGuildRow::get(stores, pool, guild_id)
            .await?
            .ok_or(TicketError::NotInSupportChannel)?;
        let support_channel_id =
            row.channel_id().ok_or(TicketError::NotInSupportChannel)?;

        let thread = support_thread(&interaction.channel, support_channel_id)?;

        let op = ThreadActivity::op_id(pool, thread.id).await?;

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

        if !may_act(interaction.user.id, op, &roles, row.role_ids(), manage) {
            return Err(TicketError::NotTicketParticipant);
        }

        let deadline = solve::mark_solved(
            http,
            app,
            stores,
            guild_id,
            &row,
            support_channel_id,
            thread.id,
        )
        .await?;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new()
                    .content(notice::solved_notice(deadline)),
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
}
