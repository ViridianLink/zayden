use futures::StreamExt;
use serenity::all::{
    CommandInteraction, CreateSelectMenu, CreateSelectMenuKind, CreateSelectMenuOption,
    EditInteractionResponse, GuildId, Http,
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
            .await
            .unwrap()
            .unwrap()
            .faq_channel_id()
            .unwrap();

        let menu_options = faq_channel_id
            .widen()
            .messages_iter(http)
            .enumerate()
            .map(|(index, msg_result)| {
                let msg = msg_result.unwrap();
                let id = msg.content.lines().next().unwrap().trim();

                CreateSelectMenuOption::new(id[2..id.len() - 2].to_string(), index.to_string())
            })
            .collect::<Vec<_>>()
            .await;

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().select_menu(CreateSelectMenu::new(
                    "support_faq",
                    CreateSelectMenuKind::String {
                        options: menu_options.into(),
                    },
                )),
            )
            .await
            .unwrap();

        Ok(())
    }
}
