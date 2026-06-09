use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    EditThread,
    GenericInteractionChannel,
    GuildId,
    Http,
};
use sqlx::{Database, Pool};

use crate::{Result, Ticket, TicketError, TicketGuildManager};

impl Ticket {
    pub(super) async fn open<Db: Database, GuildManager: TicketGuildManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        guild_id: GuildId,
    ) -> Result<()> {
        let support_channel_id = GuildManager::get(pool, guild_id)
            .await?
            .ok_or(TicketError::NotInSupportChannel)?
            .channel_id()
            .ok_or(TicketError::NotInSupportChannel)?;

        let Some(channel) = interaction.channel.as_ref() else {
            return Err(TicketError::Internal(
                "Ticket::open: interaction has no associated channel".into(),
            ));
        };

        if let GenericInteractionChannel::Thread(c) = channel
            && c.parent_id != support_channel_id
        {
            return Err(TicketError::NotInSupportChannel);
        }

        let new_channel_name = channel
            .base()
            .name
            .as_deref()
            .unwrap_or_default()
            .replace("[Fixed] - ", "")
            .replace("[Closed] - ", "");

        interaction
            .channel_id
            .expect_thread()
            .edit(http, EditThread::new().name(new_channel_name))
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
