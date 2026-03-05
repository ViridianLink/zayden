use std::collections::HashMap;

use serenity::all::{
    CommandInteraction, EditInteractionResponse, EditThread, GenericInteractionChannel, GuildId,
    Http, ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::{Error, Result, Ticket, TicketGuildManager};

impl Ticket {
    pub(super) async fn close<Db: Database, GuildManager: TicketGuildManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        mut options: HashMap<&str, ResolvedValue<'_>>,
        guild_id: GuildId,
    ) -> Result<()> {
        let message = match options.remove("message") {
            Some(ResolvedValue::String(message)) => message,
            _ => "",
        };

        match message.is_empty() {
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

        if let GenericInteractionChannel::Thread(channel) = channel
            && channel.parent_id != support_channel_id
        {
            return Err(Error::NotInSupportChannel);
        }

        let new_channel_name: String =
            format!("{} - {}", "[Closed]", channel.base().name.as_ref().unwrap())
                .chars()
                .take(100)
                .collect();

        interaction
            .channel_id
            .expect_thread()
            .edit(http, EditThread::new().name(new_channel_name))
            .await
            .unwrap();

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new()
                    .content(format!("Ticket marked as closed\n\n{message}")),
            )
            .await
            .unwrap();

        Ok(())
    }
}
