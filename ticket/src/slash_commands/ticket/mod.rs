mod close;
mod create;
mod fixed;
mod open;
mod remove;

use serenity::all::{
    CommandInteraction, CommandOptionType, CreateCommand, CreateCommandOption, Http, Permissions,
    ResolvedOption, ResolvedValue,
};
use sqlx::{Database, Pool};
use zayden_core::parse_options;

use crate::{Error, Result, Ticket, TicketGuildManager, TicketManager};

impl Ticket {
    pub async fn run<
        Db: Database,
        GuildManager: TicketGuildManager<Db>,
        Manager: TicketManager<Db>,
    >(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &Pool<Db>,
        mut options: Vec<ResolvedOption<'_>>,
    ) -> Result<()> {
        let guild_id = interaction.guild_id.ok_or(Error::MissingGuildId)?;

        let command = options.remove(0);

        let options = match command.value {
            ResolvedValue::SubCommand(options) => options,
            ResolvedValue::SubCommandGroup(options) => options,
            _ => unreachable!("Subcommand is required"),
        };
        let options = parse_options(options);

        match command.name {
            "close" => {
                Self::close::<Db, GuildManager>(http, interaction, pool, options, guild_id).await?
            }
            "create" => Self::create(http, interaction, options).await?,
            "fixed" => {
                Self::fixed::<Db, GuildManager>(http, interaction, pool, options, guild_id).await?
            }
            "open" => Self::open::<Db, GuildManager>(http, interaction, pool, guild_id).await?,
            "remove" => Self::remove::<Db, Manager>(http, interaction, pool, options).await?,
            _ => unreachable!("Subcommand is required"),
        }

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        let close =
            CreateCommandOption::new(CommandOptionType::SubCommand, "close", "Close the ticket")
                .add_sub_option(
                    CreateCommandOption::new(
                        CommandOptionType::String,
                        "message",
                        "Message to send before closing the ticket",
                    )
                    .required(false),
                );

        let create = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "create",
            "Create a ticket embed and button",
        )
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::String,
            "title",
            "The title of the ticket embed",
        ))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::String,
            "description",
            "The description of the ticket embed",
        ))
        .add_sub_option(CreateCommandOption::new(
            CommandOptionType::String,
            "label",
            "The label of the ticket button",
        ));

        let fixed = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "fixed",
            "Mark the ticket as fixed",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "version",
                "Version of the game that fixed the issue",
            )
            .required(false),
        );

        let open =
            CreateCommandOption::new(CommandOptionType::SubCommand, "open", "Open the ticket");

        let remove = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "remove",
            "Remove a ticket message",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "message",
                "The message to remove",
            )
            .required(true),
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::Channel,
                "channel",
                "The channel to remove the message from",
            )
            .required(false),
        );

        CreateCommand::new("ticket")
            .description("Ticket management commands")
            .default_member_permissions(Permissions::MANAGE_MESSAGES)
            .add_option(close)
            .add_option(create)
            .add_option(fixed)
            .add_option(open)
            .add_option(remove)
    }
}
