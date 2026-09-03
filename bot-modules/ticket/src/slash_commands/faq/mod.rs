mod ask;
mod list;

use serenity::all::{
    CommandInteraction,
    CommandOptionType,
    CreateCommandOption,
    GuildId,
    Http,
    ResolvedOption,
};
use sqlx::PgPool;
use zayden_app::state::AppState;
use zayden_core::{parse_options, parse_subcommand};

use crate::{Result, Ticket, TicketError};

impl Ticket {
    pub(super) async fn faq(
        http: &Http,
        interaction: &CommandInteraction,
        pool: &PgPool,
        app: &AppState,
        options: impl IntoIterator<Item = ResolvedOption<'_>>,
        guild_id: GuildId,
    ) -> Result<()> {
        let (name, options) = parse_subcommand(options)?;
        let options = parse_options(options);

        match name {
            "ask" => Self::faq_ask(http, interaction, app, options, guild_id).await,
            "list" => Self::faq_list(http, interaction, pool, guild_id).await,
            name => Err(TicketError::Internal(format!(
                "unrecognized faq subcommand: {name}"
            ))),
        }
    }

    pub(super) fn faq_option<'a>() -> CreateCommandOption<'a> {
        let ask = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "ask",
            "Ask the FAQ a question",
        )
        .add_sub_option(
            CreateCommandOption::new(
                CommandOptionType::String,
                "query",
                "What do you want to know?",
            )
            .required(true),
        );

        let list = CreateCommandOption::new(
            CommandOptionType::SubCommand,
            "list",
            "List this server's FAQ articles",
        );

        CreateCommandOption::new(
            CommandOptionType::SubCommandGroup,
            "faq",
            "Look up an answer",
        )
        .add_sub_option(ask)
        .add_sub_option(list)
    }
}
