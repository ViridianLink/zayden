use std::collections::HashMap;

use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    EditThread,
    GenericInteractionChannel,
    GuildId,
    Http,
    ResolvedValue,
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

        if message.is_empty() {
            interaction.defer_ephemeral(http).await?;
        } else {
            interaction.defer(http).await?;
        }

        let support_channel_id = GuildManager::get(pool, guild_id)
            .await?
            .ok_or(Error::NotInSupportChannel)?
            .channel_id()
            .ok_or(Error::NotInSupportChannel)?;

        let Some(channel) = interaction.channel.as_ref() else {
            return Ok(());
        };

        if let GenericInteractionChannel::Thread(channel) = channel
            && channel.parent_id != support_channel_id
        {
            return Err(Error::NotInSupportChannel);
        }

        let new_channel_name: String = format!(
            "[Closed] - {}",
            channel.base().name.as_deref().unwrap_or_default()
        )
        .chars()
        .take(100)
        .collect();

        interaction
            .channel_id
            .expect_thread()
            .edit(http, EditThread::new().name(new_channel_name))
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
