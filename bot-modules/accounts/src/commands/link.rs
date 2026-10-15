use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    Context,
    CreateActionRow,
    CreateButton,
    CreateCommand,
    CreateCommandOption,
    CreateComponent,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use sqlx::PgPool;
use zayden_core::{as_i64, parse_subcommand};

use crate::error::{AccountsError, Result};
use crate::links::AccountLink;
use crate::provider::Provider;

pub struct LinkCommand;

impl LinkCommand {
    pub fn register() -> CreateCommand<'static> {
        CreateCommand::new("link")
            .description("Link an external game account to your Discord account")
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "steam",
                "Link your Steam account",
            ))
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "status",
                "Show your linked accounts",
            ))
    }

    pub async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
        dashboard_url: &str,
    ) -> Result<()> {
        let (name, _) = parse_subcommand(interaction.data.options())?;

        let response = match name {
            "steam" => steam(dashboard_url),
            "status" => status(pool, interaction, dashboard_url).await?,
            _ => {
                return Err(AccountsError::Internal(format!(
                    "unknown subcommand `{name}`"
                )));
            },
        };

        interaction
            .create_response(&ctx.http, CreateInteractionResponse::Message(response))
            .await?;

        Ok(())
    }
}

fn steam(dashboard_url: &str) -> CreateInteractionResponseMessage<'static> {
    CreateInteractionResponseMessage::new()
        .content(
            "Sign in on the dashboard to link your Steam account. Steam confirms you \
             own it; Zayden never sees your password.",
        )
        .components(vec![CreateComponent::ActionRow(CreateActionRow::buttons(
            vec![
                CreateButton::new_link(format!("{dashboard_url}/account/link/steam"))
                    .label("Link Steam"),
            ],
        ))])
        .ephemeral(true)
}

async fn status(
    pool: &PgPool,
    interaction: &CommandInteraction,
    dashboard_url: &str,
) -> Result<CreateInteractionResponseMessage<'static>> {
    let links =
        AccountLink::list_for_user(pool, as_i64(interaction.user.id.get())).await?;

    let content = if links.is_empty() {
        "You have no linked accounts. Use `/link steam` to link one.".to_owned()
    } else {
        let mut lines: Vec<String> = links
            .iter()
            .map(|link| match link.provider {
                Provider::Steam => link.steam_id().map_or_else(
                    || format!("**Steam**: `{}`", link.external_id),
                    |id| {
                        format!(
                            "**Steam**: `{}` (SteamID64 `{id}`), linked <t:{}:R>",
                            id.account_id(),
                            link.linked_at.as_second()
                        )
                    },
                ),
            })
            .collect();
        lines.push(format!("Manage links: {dashboard_url}/account"));
        lines.join("\n")
    };

    Ok(CreateInteractionResponseMessage::new().content(content).ephemeral(true))
}
