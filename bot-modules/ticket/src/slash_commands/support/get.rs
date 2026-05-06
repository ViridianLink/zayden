use std::collections::HashMap;

use futures::{StreamExt, TryStreamExt};
use serenity::all::{
    CommandInteraction, CreateEmbed, EditInteractionResponse, GuildId, Http, ResolvedValue,
};
use sqlx::{Database, Pool};

use crate::{Result, Support, TicketGuildManager};

impl Support {
    pub(super) async fn get<Db: Database, GuildManager: TicketGuildManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        mut options: HashMap<&str, ResolvedValue<'_>>,
        guild_id: GuildId,
    ) -> Result<()> {
        interaction.defer(http).await?;

        let id = match options.remove("id") {
            Some(ResolvedValue::String(id)) => id,
            _ => unreachable!("ID is required"),
        };

        let faq_channel_id = GuildManager::get(pool, guild_id)
            .await
            .unwrap()
            .unwrap()
            .faq_channel_id()
            .unwrap();

        let mut stream = faq_channel_id.widen().messages_iter(http).boxed();

        while let Some(msg) = stream.try_next().await.unwrap() {
            let support_id = msg.content.lines().next().unwrap().trim();

            let title = &support_id[2..support_id.len() - 2];
            let description = msg.content.strip_prefix(support_id).unwrap();

            if support_id.contains(id) {
                interaction
                    .edit_response(
                        http,
                        EditInteractionResponse::new()
                            .embed(CreateEmbed::new().title(title).description(description)),
                    )
                    .await
                    .unwrap();

                return Ok(());
            }
        }

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().content("Support message not found"),
            )
            .await?;

        Ok(())
    }
}
