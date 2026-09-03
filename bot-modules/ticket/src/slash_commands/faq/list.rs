use serenity::all::{
    CommandInteraction,
    CreateSelectMenu,
    CreateSelectMenuKind,
    CreateSelectMenuOption,
    EditInteractionResponse,
    GuildId,
    Http,
};
use sqlx::PgPool;
use zayden_core::as_i64;

use crate::faq::FaqArticle;
use crate::{Result, Ticket};

/// Discord's per-menu option budget.
const MENU_LIMIT: i64 = 25;

const DESCRIPTION_LIMIT: usize = 100;
const EMPTY: &str = "There are no FAQ entries to list yet.";

impl Ticket {
    pub(super) async fn faq_list(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &PgPool,
        guild_id: GuildId,
    ) -> Result<()> {
        interaction.defer(http).await?;

        let articles =
            FaqArticle::list(pool, as_i64(guild_id.get()), MENU_LIMIT).await?;

        if articles.is_empty() {
            interaction
                .edit_response(http, EditInteractionResponse::new().content(EMPTY))
                .await?;

            return Ok(());
        }

        let options = articles
            .iter()
            .map(|article| {
                CreateSelectMenuOption::new(
                    article.title.clone(),
                    article.id.to_string(),
                )
                .description(
                    article
                        .summary
                        .chars()
                        .take(DESCRIPTION_LIMIT)
                        .collect::<String>(),
                )
            })
            .collect::<Vec<_>>();

        interaction
            .edit_response(
                http,
                EditInteractionResponse::new().select_menu(CreateSelectMenu::new(
                    "support_faq",
                    CreateSelectMenuKind::String { options: options.into() },
                )),
            )
            .await?;

        Ok(())
    }
}
