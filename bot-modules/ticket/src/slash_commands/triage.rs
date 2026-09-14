use serenity::all::{CommandInteraction, EditInteractionResponse, GuildId, Http};
use zayden_app::state::AppState;

use crate::faq::{FaqContext, triage_ticket};
use crate::opening::ticket_opening;
use crate::{
    Result,
    Ticket,
    TicketError,
    TicketGuildRow,
    TicketStores,
    support_thread,
};

const OPENING_ATTEMPTS: u32 = 1;

impl Ticket {
    pub(super) async fn triage(
        http: &Http,
        interaction: &CommandInteraction,
        stores: TicketStores<'_>,
        app: &AppState,
        guild_id: GuildId,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let row = TicketGuildRow::get(stores, &app.db, guild_id)
            .await?
            .ok_or(TicketError::NotInSupportChannel)?;
        let support_channel_id =
            row.channel_id().ok_or(TicketError::NotInSupportChannel)?;

        let thread_id = support_thread(&interaction.channel, support_channel_id)?.id;

        let context = FaqContext::load(stores.faq, guild_id)
            .await
            .map_err(|e| TicketError::Internal(e.to_string()))?
            .ok_or(TicketError::FaqNotConfigured)?;

        let thread = thread_id.to_thread(http, Some(guild_id)).await?;

        let opening = ticket_opening(http, &thread, OPENING_ATTEMPTS)
            .await
            .ok_or(TicketError::NothingToTriage)?;

        triage_ticket(http, app, &context, opening).await?;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().content("Triage posted"),
            )
            .await?;

        Ok(())
    }
}
