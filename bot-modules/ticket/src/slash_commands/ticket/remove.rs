use std::collections::HashMap;

use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    Http,
    MessageId,
    ResolvedValue,
};
use sqlx::{Database, Pool};
use zayden_core::{as_u64, required_option};

use crate::{Result, Ticket, TicketManager};

impl Ticket {
    pub(super) async fn remove<Db: Database, Manager: TicketManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        mut options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let id: i64 = required_option(&mut options, "message")?;

        let message_id = MessageId::new(as_u64(id));

        let channel_id = match options.remove("channel") {
            Some(ResolvedValue::Channel(channel)) => channel.id(),
            _ => interaction.channel_id,
        };

        channel_id
            .delete_message(http, message_id, Some("Deleted created ticket message"))
            .await?;

        Manager::delete(pool, message_id).await?;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().content("Message removed"),
            )
            .await?;

        Ok(())
    }
}
