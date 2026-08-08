use std::collections::HashMap;

use serenity::all::{
    CommandOptionType,
    CreateCommand,
    CreateCommandOption,
    EditInteractionResponse,
    GenericInteractionChannel,
    GuildId,
    Permissions,
    ResolvedValue,
};
use zayden_core::{InvocationCtx, parse_options, parse_subcommand, required_option};

use crate::error::{HoneypotError, Result};
use crate::settings::HoneypotSettings;

pub struct Honeypot;

impl Honeypot {
    pub fn register() -> CreateCommand<'static> {
        CreateCommand::new("honeypot")
            .description("Auto-ban spam bots that post in a decoy channel")
            .default_member_permissions(Permissions::MANAGE_GUILD)
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "set",
                    "Set the honeypot channel. Anyone who posts there is soft-banned.",
                )
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::Channel,
                        "channel",
                        "An empty channel nobody has a reason to post in",
                    )
                    .required(true),
                ),
            )
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "disable",
                "Turn the honeypot off",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "status",
                "Show the current honeypot configuration",
            ))
    }

    pub async fn run(cx: &InvocationCtx<'_>) -> Result<()> {
        let (name, sub_options) = parse_subcommand(cx.interaction.data.options())?;
        let options = parse_options(sub_options);

        let guild_id =
            cx.interaction.guild_id.ok_or(HoneypotError::MissingGuildId)?;
        require_manage_guild(cx)?;

        match name {
            "set" => set(cx, guild_id, options).await,
            "disable" => disable(cx, guild_id).await,
            "status" => status(cx, guild_id).await,
            _ => Err(HoneypotError::UnknownSubcommand(name.to_string())),
        }
    }
}

#[must_use]
pub fn is_privileged(perms: Option<Permissions>) -> bool {
    perms.is_some_and(Permissions::manage_guild)
}

fn require_manage_guild(cx: &InvocationCtx<'_>) -> Result<()> {
    let perms = cx.interaction.member.as_ref().and_then(|member| member.permissions);

    if is_privileged(perms) { Ok(()) } else { Err(HoneypotError::NotPrivileged) }
}

async fn set(
    cx: &InvocationCtx<'_>,
    guild_id: GuildId,
    mut options: HashMap<&str, ResolvedValue<'_>>,
) -> Result<()> {
    let channel: &GenericInteractionChannel =
        required_option(&mut options, "channel")?;
    let channel_id = channel.id().expect_channel();

    cx.interaction.defer_ephemeral(&cx.ctx.http).await?;

    HoneypotSettings::arm(&cx.app.settings.honeypot, guild_id, channel_id).await?;

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new().content(format!(
                "Honeypot armed on <#{channel_id}>. Anyone who posts there is \
                 banned (which purges their recent messages server-wide) and \
                 then immediately unbanned, so a recovered account can rejoin.\n\n\
                 Leave the channel postable by @everyone — the trap only catches \
                 spam bots that can actually reach it."
            )),
        )
        .await?;

    Ok(())
}

async fn disable(cx: &InvocationCtx<'_>, guild_id: GuildId) -> Result<()> {
    cx.interaction.defer_ephemeral(&cx.ctx.http).await?;

    HoneypotSettings::disarm(&cx.app.settings.honeypot, guild_id).await?;

    cx.interaction
        .edit_response(
            &cx.ctx.http,
            EditInteractionResponse::new().content("Honeypot disabled."),
        )
        .await?;

    Ok(())
}

async fn status(cx: &InvocationCtx<'_>, guild_id: GuildId) -> Result<()> {
    cx.interaction.defer_ephemeral(&cx.ctx.http).await?;

    let config = HoneypotSettings::get(&cx.app.settings.honeypot, guild_id).await?;

    let content = config.channel_id.map_or_else(
        || "Honeypot is disabled. Use `/honeypot set` to arm it.".to_string(),
        |channel_id| {
            let mut exemptions = vec!["the server owner".to_string()];
            if config.exempt_admins {
                exemptions.push("admins and Manage Server holders".to_string());
            }
            if let Some(role_id) = config.exempt_role_id {
                exemptions.push(format!("<@&{role_id}>"));
            }

            format!(
                "Honeypot is armed on <#{channel_id}>.\n**Exempt:** {}\n\n\
                 Change the exemptions from the dashboard.",
                exemptions.join(", "),
            )
        },
    );

    cx.interaction
        .edit_response(&cx.ctx.http, EditInteractionResponse::new().content(content))
        .await?;

    Ok(())
}
