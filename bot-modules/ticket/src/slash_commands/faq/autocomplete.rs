use std::sync::Arc;

use serenity::all::{
    AutocompleteChoice,
    CommandInteraction,
    CreateAutocompleteResponse,
    CreateInteractionResponse,
    Http,
    ResolvedValue,
};
use zayden_app::state::AppState;
use zayden_core::parse_subcommand;

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

        let query = focused(interaction).unwrap_or_default();

        // Autocomplete is the wrong place to report an unconfigured FAQ, so an
        // unusable config just means no suggestions.
        let Ok(Some(context)) = FaqContext::load(&app.settings.faq, guild_id).await
        else {
            return respond(http, interaction, vec![ask(query)]).await;
        };

        let choices = index.choices(guild_id, &context.wiki, query).await;

        respond(http, interaction, choices).await
    }
}

fn focused(interaction: &CommandInteraction) -> Option<&str> {
    let (_name, options) = parse_subcommand(interaction.data.options()).ok()?;

    options.iter().find_map(|option| match option.value {
        ResolvedValue::Autocomplete { value, .. } => Some(value),
        ResolvedValue::Boolean(_)
        | ResolvedValue::Integer(_)
        | ResolvedValue::Number(_)
        | ResolvedValue::String(_)
        | ResolvedValue::SubCommand(_)
        | ResolvedValue::SubCommandGroup(_)
        | ResolvedValue::Attachment(_)
        | ResolvedValue::Channel(_)
        | ResolvedValue::Role(_)
        | ResolvedValue::User(..)
        | ResolvedValue::Unresolved(_)
        | _ => None,
    })
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
