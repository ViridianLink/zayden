use serenity::all::{
    CommandInteraction, EditInteractionResponse, EditThread, GenericInteractionChannel, GuildId,
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
            .await
            .unwrap()
            .unwrap()
            .channel_id()
            .unwrap();

        let channel = interaction.channel.as_ref().unwrap();

        if let GenericInteractionChannel::Thread(c) = channel
            && c.parent_id != support_channel_id
        {
            return Err(Error::NotInSupportChannel);
        }

        let new_channel_name = channel
            .base()
            .name
            .as_ref()
            .unwrap()
            .replace("[Fixed] - ", "")
            .replace("[Closed] - ", "");

        interaction
            .channel_id
            .expect_thread()
            .edit(http, EditThread::new().name(new_channel_name))
            .await
            .unwrap();

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().content("Ticket reopened"),
            )
            .await
            .unwrap();

        Ok(())
    }
}
