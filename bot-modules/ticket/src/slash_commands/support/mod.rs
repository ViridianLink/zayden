mod get;
mod list;

use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    CreateCommand,
    CreateCommandOption,
    Http,
    Permissions,
    ResolvedOption,
};
use sqlx::{Database, Pool};
use zayden_core::{CoreError as ZaydenError, parse_options, parse_subcommand};

use crate::{Result, Support, TicketError, TicketGuildManager};

impl Support {
    pub async fn run<Db: Database, GuildManager: TicketGuildManager<Db>>(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        options: Vec<ResolvedOption<'_>>,
    ) -> Result<()> {
        let guild_id = interaction.guild_id.ok_or(ZaydenError::MissingGuildId)?;

        let (name, options) = parse_subcommand(options)?;
        let options = parse_options(options);

        match name {
            "get" => {
                Self::get::<Db, GuildManager>(
                    http,
                    interaction,
                    pool,
                    options,
                    guild_id,
                )
                .await?;
            },
            "list" => {
                Self::list::<Db, GuildManager>(http, interaction, pool, guild_id)
                    .await?;
            },
            _ => {
                return Err(TicketError::Internal(format!(
                    "unexpected subcommand: {name}"
                )));
            },
        }

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        let list = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "list",
            "List all support messages",
        );
        let get = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "get",
            "Get a support message",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "id",
                "The ID of the support message",
            )
            .required(true),
        );

        CreateCommand::new("support")
            .description("Support FAQ commands")
            .default_member_permissions(Permissions::MANAGE_MESSAGES)
            .add_option(get)
            .add_option(list)
    }
}
