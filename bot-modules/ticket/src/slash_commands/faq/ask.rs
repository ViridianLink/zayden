use std::collections::HashMap;

use serenity::all::{
    CommandInteraction,
    EditInteractionResponse,
    GuildId,
    Http,
    ResolvedValue,
};
use tracing::{error, warn};
use zayden_app::state::AppState;
use zayden_core::required_option;

use crate::faq::{FaqContext, answer, embeds};
use crate::wiki::{self, Page, WikiConfig};
use crate::{Result, Ticket, TicketError};

impl Ticket {
    pub(super) async fn faq_ask(
        http: &Http,
        interaction: &CommandInteraction,
        app: &AppState,
        mut options: HashMap<&str, ResolvedValue<'_>>,
        guild_id: GuildId,
    ) -> Result<()> {
        interaction.defer(http).await?;

        let query: &str = required_option(&mut options, "query")?;

        let context = FaqContext::load(&app.settings.faq, guild_id)
            .await
            .map_err(|e| TicketError::Internal(e.to_string()))?
            .ok_or(TicketError::FaqNotConfigured)?;

        let results = wiki::search(&app.http, &context.wiki, query)
            .await
            .map_err(|e| TicketError::Wiki(e.to_string()))?;

        let results =
            results.into_iter().take(context.wiki.max_results()).collect::<Vec<_>>();

        let Some(top) = results.first() else {
            interaction
                .edit_response(
                    http,
                    EditInteractionResponse::new()
                        .embed(embeds::results(&context.wiki, &results)),
                )
                .await?;

            return Ok(());
        };

        let Some(page) = fetch(app, &context.wiki, &top.path).await else {
            interaction
                .edit_response(
                    http,
                    EditInteractionResponse::new()
                        .embed(embeds::results(&context.wiki, &results)),
                )
                .await?;

            return Ok(());
        };

        let embed = match answer(app, context.tuning, query, &page).await {
            Ok(answer) => embeds::answer(&context.wiki, &page, &answer),
            Err(e) => {
                error!(error = ?e, query, %guild_id, "faq answer failed");
                embeds::page(&context.wiki, &page)
            },
        };

        interaction
            .edit_response(http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }
}

async fn fetch(app: &AppState, config: &WikiConfig, path: &str) -> Option<Page> {
    match wiki::page(&app.http, config, path).await {
        Ok(page) => Some(page),
        Err(e) => {
            warn!(error = ?e, path, "could not read wiki page source");
            None
        },
    }
}
