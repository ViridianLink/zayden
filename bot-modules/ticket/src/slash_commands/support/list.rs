use futures::StreamExt;
use serenity::all::{
    CommandInteraction,
    CreateSelectMenu,
    CreateSelectMenuKind,
    CreateSelectMenuOption,
    EditInteractionResponse,
    GuildId,
    Http,
};
use sqlx::{Database, Pool};

use crate::{Result, Support, TicketGuildManager};

impl Support {
    pub(super) async fn list<Db: Database, GuildManager: TicketGuildManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        guild_id: GuildId,
    ) -> Result<()> {
        let faq_channel_id = GuildManager::get(pool, guild_id)
            .await?
            .ok_or(crate::Error::SupportNotFound)?
            .faq_channel_id()
            .ok_or(crate::Error::SupportNotFound)?;

        let menu_options = faq_channel_id
            .widen()
            .messages_iter(http)
            .enumerate()
            .map(|(index, msg_result)| {
                let msg = msg_result
                    .expect("message fetch always succeeds in messages_iter");
                let id = msg
                    .content
                    .lines()
                    .next()
                    .expect("message content always has at least one line")
                    .trim();

                CreateSelectMenuOption::new(
                    id.get(2..id.len().saturating_sub(2)).unwrap_or(id).to_string(),
                    index.to_string(),
                )
            })
            .collect::<Vec<_>>()
            .await;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().select_menu(CreateSelectMenu::new(
                    "support_faq",
                    CreateSelectMenuKind::String { options: menu_options.into() },
                )),
            )
            .await?;

        Ok(())
    }
}
