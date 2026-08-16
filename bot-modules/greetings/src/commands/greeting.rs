use rand::rng;
use rand::seq::IndexedRandom;
use serenity::all::{
    CommandOptionType,
    CreateAttachment,
    CreateCommand,
    CreateCommandOption,
    EditInteractionResponse,
    User,
};
use zayden_app::config::GreetingsSettingsRow;
use zayden_core::{
    InvocationCtx,
    optional_option,
    parse_options,
    parse_subcommand,
    server_tier,
};

use crate::attachment;
use crate::cooldown::{COOLDOWNS, Verdict};
use crate::error::{GreetingsError, Result};
use crate::images::GreetingImage;
use crate::kind::GreetingKind;
use crate::settings::{GreetingsConfig, GreetingsSettings, GreetingsStore, render};

async fn check_cooldown(
    cx: &InvocationCtx<'_>,
    config: &GreetingsConfig,
) -> Result<()> {
    let Some(guild_id) = cx.interaction.guild_id else {
        return Err(GreetingsError::GuildOnly);
    };

    let tier = server_tier(&cx.ctx.http, &cx.app.entitlements, guild_id).await;
    let limits = config.cooldowns.clamp_to(GreetingsSettingsRow::floors_for(tier));

    match COOLDOWNS.check_and_record(guild_id, cx.interaction.user.id, limits).await
    {
        Verdict::Allowed => Ok(()),
        Verdict::UserWait(secs) => Err(GreetingsError::UserCooldown(secs)),
        Verdict::GuildWait(secs) => Err(GreetingsError::GuildCooldown(secs)),
    }
}

async fn attach(
    cx: &InvocationCtx<'_>,
    url: &str,
    kind: GreetingKind,
) -> Option<CreateAttachment<'static>> {
    match attachment::fetch(&cx.app.http, &cx.app.discord_token, url, kind).await {
        Ok(attachment) => Some(attachment),
        Err(error) => {
            tracing::warn!(
                url,
                %error,
                "greeting image could not be attached; sending text only"
            );
            None
        },
    }
}

pub async fn run(cx: &InvocationCtx<'_>, store: &GreetingsStore) -> Result<()> {
    let interaction = cx.interaction;
    let http = &cx.ctx.http;
    let guild_id = interaction.guild_id.ok_or(GreetingsError::GuildOnly)?;

    let (name, sub_options) = parse_subcommand(interaction.data.options())?;
    let kind = GreetingKind::parse(name)?;

    let config = GreetingsSettings::get(store, guild_id).await?;

    check_cooldown(cx, &config).await?;

    interaction.defer(http).await?;

    let mut options = parse_options(sub_options);

    let target = optional_option::<(&User, _), _>(&mut options, "user")
        .map_or(&interaction.user, |(user, _)| user);

    let images = GreetingImage::list(&cx.app.db, guild_id, kind).await?;

    let image = {
        let mut rng = rng();
        images.choose(&mut rng).map(|image| image.url.clone())
    };

    let attachment = match image.as_deref() {
        None => None,
        Some(url) => attach(cx, url, kind).await,
    };

    let template = match (config.message_for(kind), attachment.as_ref()) {
        (Some(message), _) => Some(message),
        (None, None) => Some(kind.default_message()),
        (None, Some(_)) => None,
    };

    let mut response = EditInteractionResponse::new();

    if let Some(template) = template {
        response =
            response.content(render(template, target.id, interaction.user.id));
    }

    if let Some(attachment) = attachment {
        response = response.new_attachment(attachment);
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
