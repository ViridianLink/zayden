use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    CreateCommand,
    CreateCommandOption,
    EditInteractionResponse,
    Http,
    Permissions,
    ReactionType,
    ResolvedValue,
};
use sqlx::{Database, Pool};
use zayden_core::{parse_subcommand, required_option};

mod add;
mod remove;

use crate::error::{Error, Result};
use crate::reaction_roles_manager::ReactionRolesManager;

pub struct ReactionRoleCommand;

impl ReactionRoleCommand {
    pub async fn run<Db: Database, Manager: ReactionRolesManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction.defer_ephemeral(http).await?;

        let guild_id = interaction.guild_id.ok_or(Error::MissingGuildId)?;

        let (name, mut options) = parse_subcommand(interaction.data.options())?;

        let channel_id = match options.remove("channel") {
            Some(ResolvedValue::Channel(channel)) => channel.id(),
            _ => interaction.channel_id,
        };

        let emoji: &str = required_option(&mut options, "emoji")?;

        let reaction = ReactionType::try_from(emoji)?;

        match name {
            "add" => {
                Self::add::<Db, Manager>(
                    http, pool, guild_id, channel_id, reaction, options,
                )
                .await?;
            },
            "remove" => {
                Self::remove::<Db, Manager>(
                    http, pool, channel_id, guild_id, reaction, options,
                )
                .await?;
            },
            _ => return Ok(()),
        }

        interaction
            .edit_response(http, EditInteractionResponse::new().content("Success."))
            .await?;

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        let add = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "add",
            "Adds a reaction role",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "emoji",
                "The emoji of the reaction role",
            )
            .required(true),
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Role,
                "role",
                "The role to add when the emoji is reacted to",
            )
            .required(true),
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Channel,
            "channel",
            "The channel the message is in",
        ))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::String,
            "message_id",
            "The message id of the reaction role message",
        ));

        let remove = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "remove",
            "Removes a reaction role",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "message_id",
                "The message id of the reaction role message",
            )
            .required(true),
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "emoji",
                "The emoji of the reaction role",
            )
            .required(true),
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::Channel,
            "channel",
            "The channel the message is in",
        ));

        CreateCommand::new("reaction_role")
            .description("Adds or removes a reaction role")
            .default_member_permissions(Permissions::MANAGE_MESSAGES)
            .add_option(add)
            .add_option(remove)
    }
}
