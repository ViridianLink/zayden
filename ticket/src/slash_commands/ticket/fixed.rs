use std::collections::HashMap;

use serenity::all::{
    CommandInteraction, EditInteractionResponse, EditThread, GenericInteractionChannel, GuildId,
    Http, ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::{Error, Result, Ticket, TicketGuildManager};

impl Ticket {
    pub(super) async fn fixed<Db: Database, GuildManager: TicketGuildManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        mut options: HashMap<&str, ResolvedValue<'_>>,
        guild_id: GuildId,
    ) -> Result<()> {
        let version = match options.remove("version") {
            Some(ResolvedValue::String(message)) => message,
            _ => "",
        };

        match version.is_empty() {
            true => interaction.defer_ephemeral(http).await?,
            false => interaction.defer(http).await?,
        };

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

        let new_channel_name = format!("{} - {}", "[Fixed]", channel.base().name.as_ref().unwrap())
            .chars()
            .take(100)
            .collect::<String>();

        interaction
            .channel_id
            .expect_thread()
            .edit(http, EditThread::new().name(new_channel_name))
            .await
            .unwrap();

        let response = if version.is_empty() {
            EditInteractionResponse::new().content("Ticket marked as fixed")
        } else {
            EditInteractionResponse::new().content(format!("Ticket marked as fixed for {version}"))
        };

        interaction.edit_response(http, response).await?;

        Ok(())
    }
}
