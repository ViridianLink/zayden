use std::borrow::Cow;

use rand::rng;
use rand::seq::IndexedRandom;
use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    CreateCommand,
    CreateCommandOption,
    CreateEmbed,
    EditInteractionResponse,
    Http,
    ResolvedOption,
    User,
};
use sqlx::PgPool;
use zayden_core::{optional_option, parse_options, parse_subcommand};

use crate::error::{GreetingsError, Result};
use crate::images::GreetingImage;
use crate::kind::GreetingKind;
use crate::settings::{GreetingsSettings, GreetingsStore, render};

pub async fn run(
    http: &Http,
    interaction: &CommandInteraction,
    options: Vec<ResolvedOption<'_>>,
    pool: &PgPool,
    store: &GreetingsStore,
) -> Result<()> {
    let guild_id = interaction.guild_id.ok_or(GreetingsError::GuildOnly)?;

    let (name, sub_options) = parse_subcommand(options)?;
    let kind = GreetingKind::parse(name)?;

    interaction.defer(http).await?;

    let mut options = parse_options(sub_options);

    let target = optional_option::<(&User, _), _>(&mut options, "user")
        .map_or(&interaction.user, |(user, _)| user);

    let config = GreetingsSettings::get(store, guild_id).await?;
    let images = GreetingImage::list(pool, guild_id, kind).await?;

    let image = {
        let mut rng = rng();
        images.choose(&mut rng).map(|image| image.url.clone())
    };

    let template = match (config.message_for(kind), image.as_deref()) {
        (Some(message), _) => Some(message),
        (None, None) => Some(kind.default_message()),
        (None, Some(_)) => None,
    };

    let mut response = EditInteractionResponse::new();

    if let Some(template) = template {
        response =
            response.content(render(template, target.id, interaction.user.id));
    }

    if let Some(url) = image {
        response = response.embed(
            CreateEmbed::new().image(url, Some(Cow::Borrowed(kind.image_alt()))),
        );
    }

    interaction.edit_response(http, response).await?;

    Ok(())
}

fn subcommand<'a>(kind: GreetingKind) -> CreateCommandOption<'a> {
    CreateCommandOption::new(
        CommandOptionType::SubCommand,
        kind.subcommand_name(),
        kind.subcommand_description(),
    )
    .add_sub_option(CreateCommandOption::new(
        CommandOptionType::User,
        "user",
        "Who to greet (defaults to you)",
    ))
}

pub fn register<'a>() -> CreateCommand<'a> {
    CreateCommand::new("good")
        .description("Wish someone a good morning or good night")
        .add_option(subcommand(GreetingKind::Morning))
        .add_option(subcommand(GreetingKind::Night))
}
