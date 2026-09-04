use serenity::all::{CommandInteraction, EditInteractionResponse, GuildId, Http};
use sqlx::PgPool;

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
    pub(super) async fn open(
        http: &Http,
        interaction: &CommandInteraction,
        stores: TicketStores<'_>,
        pool: &PgPool,
        guild_id: GuildId,
    ) -> Result<()> {
        interaction.defer(http).await?;

        let row = TicketGuildRow::get(stores, pool, guild_id)
            .await?
            .ok_or(TicketError::NotInSupportChannel)?;
        let support_channel_id =
            row.channel_id().ok_or(TicketError::NotInSupportChannel)?;

        let thread = support_thread(&interaction.channel, support_channel_id)?;

        state::clear(
            http,
            guild_id,
            support_channel_id,
            thread,
            &row.state_tag_ids(),
        )
        .await?;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().content("Ticket reopened"),
            )
            .await?;

        Ok(())
    }
}
