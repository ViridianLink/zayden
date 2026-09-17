use std::sync::Arc;

use serenity::all::{
    AutocompleteChoice,
    CommandInteraction,
    CreateAutocompleteResponse,
    CreateInteractionResponse,
    Http,
};
use zayden_app::state::AppState;

use crate::faq::index::choice::ask;
use crate::faq::{FaqContext, WikiIndex};
use crate::{Result, Ticket};

impl Ticket {
    pub async fn faq_autocomplete(
        http: &Http,
        interaction: &CommandInteraction,
        app: &AppState,
        index: &Arc<WikiIndex>,
    ) -> Result<()> {
        let Some(guild_id) = interaction.guild_id else {
            return respond(http, interaction, Vec::new()).await;
        };

        let query =
            interaction.data.autocomplete().map_or("", |option| option.value);

        // Autocomplete is the wrong place to report an unconfigured FAQ, so an
        // unusable config just means no suggestions.
        let Ok(Some(context)) = FaqContext::load(&app.settings.faq, guild_id).await
        else {
            return respond(http, interaction, ask(query).into_iter().collect())
                .await;
        };

        let choices = index.choices(guild_id, &context.wiki, query);

        respond(http, interaction, choices).await
    }
}

async fn respond(
    http: &Http,
    interaction: &CommandInteraction,
    choices: Vec<AutocompleteChoice<'static>>,
) -> Result<()> {
    interaction
        .create_response(
            http,
            CreateInteractionResponse::Autocomplete(
                CreateAutocompleteResponse::new().set_choices(choices),
            ),
        )
        .await?;

    Ok(())
}
