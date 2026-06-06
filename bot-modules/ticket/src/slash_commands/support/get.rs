use std::collections::HashMap;

use futures::{StreamExt, TryStreamExt};
use serenity::all::{
    CommandInteraction,
    CreateEmbed,
    EditInteractionResponse,
    GuildId,
    Http,
    ResolvedValue,
};
use sqlx::{Database, Pool};
use zayden_core::required_option;

use crate::{Error, Result, Support, TicketGuildManager};

impl Support {
    pub(super) async fn get<Db: Database, GuildManager: TicketGuildManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        mut options: HashMap<&str, ResolvedValue<'_>>,
        guild_id: GuildId,
    ) -> Result<()> {
        interaction.defer(http).await?;

        let id: &str = required_option(&mut options, "id")?;

        let faq_channel_id = GuildManager::get(pool, guild_id)
            .await?
            .ok_or(Error::SupportNotFound)?
            .faq_channel_id()
            .ok_or(Error::SupportNotFound)?;

        let mut stream = faq_channel_id.widen().messages_iter(http).boxed();

        while let Some(msg) = stream.try_next().await? {
            let support_id = msg
                .content
                .lines()
                .next()
                .expect("message content always has at least one line")
                .trim();

            let title = support_id
                .get(2..support_id.len().saturating_sub(2))
                .unwrap_or(support_id);
            let description = msg
                .content
                .strip_prefix(support_id)
                .expect("content starts with support_id");

            if support_id.contains(id) {
                interaction
                    .edit_response(
                        http,
                        EditInteractionResponse::new().embed(
                            CreateEmbed::new().title(title).description(description),
                        ),
                    )
                    .await?;

                return Ok(());
            }
        }

        Err(Error::SupportNotFound)
    }
}
