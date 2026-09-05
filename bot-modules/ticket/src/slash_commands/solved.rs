use std::sync::Arc;

use jiff::Timestamp;
use serenity::all::{
    CommandInteraction,
    CreateInteractionResponseFollowup,
    EditInteractionResponse,
    GuildId,
    Http,
};
use zayden_app::state::AppState;

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

        solve::mark_solved(
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
                EditInteractionResponse::new().content(solve::solved_notice(
                    Timestamp::now(),
                    row.solved_archive_secs,
                )),
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
