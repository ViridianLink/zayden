use serenity::all::{
    ButtonStyle,
    CommandInteraction,
    CommandOptionType,
    ComponentInteraction,
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

pub const UNLINK_CONFIRM_PREFIX: &str = "accounts_unlink:";
pub const UNLINK_CANCEL_ID: &str = "accounts_unlink_cancel";

pub struct UnlinkCommand;

impl UnlinkCommand {
    pub fn register() -> CreateCommand<'static> {
        CreateCommand::new("unlink")
            .description("Unlink an external game account from your Discord account")
            .add_option(CreateCommandOption::new(
                CommandOptionType::SubCommand,
                "steam",
                "Unlink your Steam account",
            ))
    }

    pub async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let (name, _) = parse_subcommand(interaction.data.options())?;
        let provider: Provider = name.parse()?;

        AccountLink::select(pool, as_i64(interaction.user.id.get()), provider)
            .await?
            .ok_or(AccountsError::NotLinked(provider.label()))?;

        let confirm_id = format!("{UNLINK_CONFIRM_PREFIX}{}", provider.as_str());

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content(format!(
                            "Unlink your {} account? Any game data Zayden tracks for \
                             it will be deleted.",
                            provider.label()
                        ))
                        .components(vec![CreateComponent::ActionRow(
                            CreateActionRow::buttons(vec![
                                CreateButton::new(confirm_id)
                                    .label("Unlink")
                                    .style(ButtonStyle::Danger),
                                CreateButton::new(UNLINK_CANCEL_ID)
                                    .label("Cancel")
                                    .style(ButtonStyle::Secondary),
                            ]),
                        )])
                        .ephemeral(true),
                ),
            )
            .await?;

        Ok(())
    }

    pub async fn confirm(
        ctx: &Context,
        interaction: &ComponentInteraction,
        pool: &PgPool,
    ) -> Result<()> {
        let custom_id: &str = &interaction.data.custom_id;
        let provider: Provider = custom_id
            .strip_prefix(UNLINK_CONFIRM_PREFIX)
            .ok_or_else(|| AccountsError::UnknownProvider(custom_id.to_owned()))?
            .parse()?;

        let removed =
            AccountLink::delete(pool, as_i64(interaction.user.id.get()), provider)
                .await?;

        let content = if removed {
            format!("Unlinked your {} account.", provider.label())
        } else {
            format!("No {} account was linked.", provider.label())
        };

        update(ctx, interaction, content).await
    }

    pub async fn cancel(
        ctx: &Context,
        interaction: &ComponentInteraction,
    ) -> Result<()> {
        update(ctx, interaction, "Cancelled.".to_owned()).await
    }
}

async fn update(
    ctx: &Context,
    interaction: &ComponentInteraction,
    content: String,
) -> Result<()> {
    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::new()
                    .content(content)
                    .components(Vec::new()),
            ),
        )
        .await?;

    Ok(())
}
