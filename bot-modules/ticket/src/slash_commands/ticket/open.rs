use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    EditThread,
    GenericInteractionChannel,
    GuildId,
    Http,
};
use sqlx::{Database, Pool};

use crate::{Error, Result, Ticket, TicketGuildManager};

impl Ticket {
    pub(super) async fn open<Db: Database, GuildManager: TicketGuildManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        guild_id: GuildId,
    ) -> Result<()> {
        let support_channel_id = GuildManager::get(pool, guild_id)
            .await?
            .ok_or(Error::NotInSupportChannel)?
            .channel_id()
            .ok_or(Error::NotInSupportChannel)?;

        let channel =
            interaction.channel.as_ref().expect("interaction always has a channel");

        if let GenericInteractionChannel::Thread(c) = channel
            && c.parent_id != support_channel_id
        {
            return Err(Error::NotInSupportChannel);
        }

        let new_channel_name = channel
            .base()
            .name
            .as_ref()
            .expect("channel always has a name")
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
