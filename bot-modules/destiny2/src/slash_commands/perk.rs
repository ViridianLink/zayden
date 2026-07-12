use serenity::all::{
    AutocompleteChoice,
    AutocompleteOption,
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateAutocompleteResponse,
    CreateCommandOption,
    CreateEmbed,
    CreateEmbedFooter,
    CreateInteractionResponse,
    EditInteractionResponse,
    Http,
    ResolvedOption,
};
use sqlx::PgPool;
use zayden_core::sole_option;

use crate::db::compendium as perk_db;
use crate::{DestinyError, Result, compendium};

pub struct Perk;

impl Perk {
    pub async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        mut options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
        api_key: &str,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await?;

        let perk: &str = sole_option(&mut options)?;

        if perk_db::is_empty(pool).await? {
            compendium::update(pool, api_key).await?;
        }
        let Some(perk) = perk_db::find(pool, &perk.to_lowercase()).await? else {
            return Err(DestinyError::PerkNotFound(perk.to_string()));
        };

        let embed = CreateEmbed::new()
            .title(perk.name)
            .description(perk.description)
            .footer(CreateEmbedFooter::new("From 'Destiny Data Compendium'"));

        interaction
            .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommandOption<'a> {
        CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "perk",
            "Perk information",
        )
        .add_sub_option(
            CreateCommandOption::new(CommandOptionType::String, "perk", "Perk name")
                .required(true)
                .set_autocomplete(true),
        )
    }

    pub async fn autocomplete(
        http: &Http,
        interaction: &CommandInteraction,
        option: AutocompleteOption<'_>,
        pool: &PgPool,
        api_key: &str,
    ) -> Result<()> {
        if perk_db::is_empty(pool).await? {
            compendium::update(pool, api_key).await?;
        }

        let perks = perk_db::search(pool, &option.value.to_lowercase())
            .await?
            .into_iter()
            .map(|perk| AutocompleteChoice::from(perk.name))
            .collect::<Vec<_>>();

        interaction
            .create_response(
                http,
                CreateInteractionResponse::Autocomplete(
                    CreateAutocompleteResponse::new().set_choices(perks),
                ),
            )
            .await?;

        Ok(())
    }
}
