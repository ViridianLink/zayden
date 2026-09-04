use std::collections::HashMap;

use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    GuildId,
    Http,
    ResolvedValue,
};
use sqlx::PgPool;

use crate::idle::ThreadActivity;
use crate::{
    Result,
    Ticket,
    TicketError,
    TicketGuildRow,
    TicketStores,
    state,
    support_thread,
};

impl Ticket {
    pub(super) async fn close(
        http: &Http,
        interaction: &CommandInteraction,
        stores: TicketStores<'_>,
        pool: &PgPool,
        mut options: HashMap<&str, ResolvedValue<'_>>,
        guild_id: GuildId,
    ) -> Result<()> {
        let message = match options.remove("message") {
            Some(ResolvedValue::String(message)) => message,
            _ => "",
        };

        if message.is_empty() {
            interaction.defer_ephemeral(http).await?;
        } else {
            interaction.defer(http).await?;
        }

        let row = TicketGuildRow::get(stores, pool, guild_id)
            .await?
            .ok_or(TicketError::NotInSupportChannel)?;
        let support_channel_id =
            row.channel_id().ok_or(TicketError::NotInSupportChannel)?;

        let thread = support_thread(&interaction.channel, support_channel_id)?;

        ThreadActivity::pause(pool, thread.id).await?;

        state::mark(
            http,
            guild_id,
            support_channel_id,
            thread.id,
            row.closed_tag_id(),
            state::CLOSED,
        )
        .await?;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new()
                    .content(format!("Ticket marked as closed\n\n{message}")),
            )
            .await?;

        Ok(())
    }
}
