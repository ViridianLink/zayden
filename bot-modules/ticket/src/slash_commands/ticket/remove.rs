use std::collections::HashMap;

use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    Http,
    MessageId,
    ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::{Result, Ticket, TicketManager};

impl Ticket {
    pub(super) async fn remove<Db: Database, Manager: TicketManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        mut options: HashMap<&str, ResolvedValue<'_>>,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        #[expect(
            clippy::unreachable,
            reason = "Discord guarantees required options are present"
        )]
        let message_id = match options.remove("message") {
            Some(ResolvedValue::Integer(id)) => MessageId::new(id.cast_unsigned()),
            _ => unreachable!("ID is required"),
        };

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
