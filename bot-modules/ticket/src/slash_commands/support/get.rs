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
use sqlx::PgPool;
use zayden_core::required_option;

use crate::{Result, Support, TicketError, TicketGuildRow, TicketStores};

impl Support {
    pub(super) async fn get(
        http: &Http,
        interaction: &CommandInteraction,
        stores: TicketStores<'_>,
        pool: &PgPool,
        mut options: HashMap<&str, ResolvedValue<'_>>,
        guild_id: GuildId,
    ) -> Result<()> {
        interaction.defer(http).await?;

        let id: &str = required_option(&mut options, "id")?;

        let faq_channel_id = TicketGuildRow::get(stores, pool, guild_id)
            .await?
            .ok_or(TicketError::SupportNotFound)?
            .faq_channel_id()
            .ok_or(TicketError::SupportNotFound)?;

        let mut stream = faq_channel_id.widen().messages_iter(http).boxed();

        while let Some(msg) = stream.try_next().await? {
            let Some(first_line) = msg.content.lines().next() else {
                continue;
            };
            let support_id = first_line.trim();

            let title = support_id
                .get(2..support_id.len().saturating_sub(2))
                .unwrap_or(support_id);
            let description =
                msg.content.strip_prefix(first_line).unwrap_or_default();

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

        Err(TicketError::SupportNotFound)
    }
}
