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
use zayden_core::limits::select_menu::{
    OPTION_DESCRIPTION as MAX_OPTION_DESCRIPTION,
    OPTIONS as MAX_OPTIONS,
};

use crate::faq::FaqArticle;
use crate::{Result, Ticket};

const EMPTY: &str = "There are no FAQ entries to list yet.";

impl Ticket {
    pub(super) async fn faq_list(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &PgPool,
        guild_id: GuildId,
    ) -> Result<()> {
        interaction.defer(http).await?;

        let articles = FaqArticle::list(
            pool,
            as_i64(guild_id.get()),
            i64::try_from(MAX_OPTIONS).unwrap_or(i64::MAX),
        )
        .await?;

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
                        .take(MAX_OPTION_DESCRIPTION)
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
