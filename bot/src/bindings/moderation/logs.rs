use async_trait::async_trait;
use serenity::all::{
    CommandInteraction, CommandOptionType, Context, CreateCommand, CreateCommandOption,
    CreateEmbed, EditInteractionResponse, Permissions, Ready, ResolvedOption, ResolvedValue, User,
};
use sqlx::{PgPool, Postgres};
use core::{SlashCommand, parse_options, required_option};

use crate::{Error, Result};

use super::InfractionRow;

pub struct Logs;

#[async_trait]
impl SlashCommand<Error, Postgres> for Logs {
    async fn run(&self,
        ctx: &Context,
        interaction: &CommandInteraction,
        options: Vec<ResolvedOption<'_>>,
        pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(ctx).await?;

        let mut options = parse_options(options);

        let user: &User = required_option(&mut options, "user")?;

        let filter = match options.remove("filter") {
            Some(ResolvedValue::String(filter)) => filter,
            _ => "recent",
        };

        let infractions =
            InfractionRow::user_infractions(pool, user.id, filter == "recent").await?;

        let fields = infractions.into_iter().map(|infraction| {
            (
                format!("Case #{}", infraction.id),
                format!("**Type:** {}\n", infraction.infraction_type)
                    + &format!(
                        "**User:** ({}) {}\n",
                        infraction.user_id, infraction.username
                    )
                    + &format!(
                        "**Moderator:** ({}) {}\n",
                        infraction.moderator_id, infraction.moderator_username
                    )
                    + &format!("**Reason:** {}", infraction.reason),
                false,
            )
        });

        let embed = CreateEmbed::new()
            .title(format!("Logs for {}", user.name))
            .fields(fields);

        interaction
            .edit_response(ctx, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand> {
        let command = CreateCommand::new("logs")
            .description("Get logs for a user")
            .default_member_permissions(Permissions::MODERATE_MEMBERS)
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::User,
                    "user",
                    "The user to get logs for",
                )
                .required(true),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "filter",
                    "The number of logs to get",
                )
                .add_string_choice("Recent (default)", "recent")
                .add_string_choice("All", "all"),
            );

        Ok(command)
    }
}
