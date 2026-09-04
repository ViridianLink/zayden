mod close;
mod create;
mod faq;
mod open;
mod remove;
mod solved;

use std::sync::Arc;

use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    CreateCommand,
    CreateCommandOption,
    Http,
    Permissions,
    ResolvedOption,
};
use zayden_app::state::AppState;
use zayden_core::{CoreError as ZaydenError, parse_options, parse_subcommand};

use crate::{Result, Ticket, TicketError, TicketStores};

fn require_manage(interaction: &CommandInteraction) -> Result<()> {
    let allowed = interaction
        .member
        .as_ref()
        .and_then(|member| member.permissions)
        .is_some_and(|permissions| {
            permissions.contains(Permissions::MANAGE_MESSAGES)
        });

    if allowed { Ok(()) } else { Err(TicketError::MissingPermissions) }
}

impl Ticket {
    pub async fn run(
        http: &Arc<Http>,
        interaction: &CommandInteraction,
        app: &Arc<AppState>,
        options: Vec<ResolvedOption<'_>>,
    ) -> Result<()> {
        let stores = TicketStores::from_app(app);
        let pool = &app.db;

        let guild_id = interaction.guild_id.ok_or(ZaydenError::MissingGuildId)?;

        let (name, options) = parse_subcommand(options)?;

        if name == "faq" {
            return Self::faq(http, interaction, pool, app, options, guild_id).await;
        }

        require_manage(interaction)?;

        let options = parse_options(options);

        match name {
            "close" => {
                Self::close(http, interaction, stores, pool, options, guild_id)
                    .await?;
            },
            "create" => Self::create(http, interaction, options).await?,
            "open" => {
                Self::open(http, interaction, stores, pool, guild_id).await?;
            },
            "remove" => {
                Self::remove(http, interaction, pool, options).await?;
            },
            "solved" => {
                Self::solved(http, interaction, stores, app, guild_id).await?;
            },
            name => {
                return Err(TicketError::Internal(format!(
                    "unrecognized ticket subcommand: {name}"
                )));
            },
        }

        Ok(())
    }

    pub fn register<'a>() -> CreateCommand<'a> {
        let close = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "close",
            "Close the ticket",
        )
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

        let open = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "open",
            "Open the ticket",
        );

        let solved = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "solved",
            "Mark the ticket as solved",
        );

        CreateCommand::new("ticket")
            .description("Ticket and FAQ commands")
            .add_option(Self::faq_option())
            .add_option(close)
            .add_option(create)
            .add_option(open)
            .add_option(solved)

        // CreateCommand::new("Ticket Remove").kind(CommandType::Message),
    }
}
