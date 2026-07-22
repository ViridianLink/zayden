use async_trait::async_trait;
use serenity::all::{
    CommandOptionType,
    CreateCommand,
    CreateCommandOption,
    CreateEmbed,
    EditInteractionResponse,
    Permissions,
    ResolvedValue,
    User,
};
use zayden_core::{
    HandlerError,
    InvocationCtx,
    ModuleCommand,
    parse_options,
    required_option,
};

use super::InfractionRow;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum LogFilter {
    #[default]
    Recent,
    All,
}

impl LogFilter {
    const ALL: &'static str = "all";
    const RECENT: &'static str = "recent";

    const fn is_recent(self) -> bool {
        matches!(self, Self::Recent)
    }
}

impl From<&str> for LogFilter {
    fn from(value: &str) -> Self {
        match value {
            Self::ALL => Self::All,
            _ => Self::Recent,
        }
    }
}

pub(super) struct Logs;

#[async_trait]
impl ModuleCommand for Logs {
    fn module(&self) -> Option<&'static str> {
        Some("moderation")
    }

    fn definition(&self) -> CreateCommand<'static> {
        CreateCommand::new("logs")
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
                .add_string_choice("Recent", LogFilter::RECENT)
                .add_string_choice("All", LogFilter::ALL),
            )
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        cx.interaction.defer(&cx.ctx.http).await?;

        let options = cx.interaction.data.options();
        let mut options = parse_options(options);

        let user: &User = required_option(&mut options, "user")?;

        let filter = match options.remove("filter") {
            Some(ResolvedValue::String(filter)) => LogFilter::from(filter),
            _ => LogFilter::default(),
        };

        let infractions =
            InfractionRow::user_infractions(&cx.app.db, user.id, filter.is_recent())
                .await?;

        if infractions.is_empty() {
            cx.interaction
                .edit_response(
                    &cx.ctx.http,
                    EditInteractionResponse::new().content(format!(
                        "{} has no infractions on record.",
                        user.name
                    )),
                )
                .await?;

            return Ok(());
        }

        let fields = infractions.into_iter().map(|infraction| {
            (
                format!("Case #{}", infraction.id),
                format!(
                    "**Type:** {}\n**User:** ({}) {}\n**Moderator:** ({}) {}\n**Reason:** {}",
                    infraction.infraction_type,
                    infraction.user_id,
                    infraction.username,
                    infraction.moderator_id,
                    infraction.moderator_username,
                    infraction.reason,
                ),
                false,
            )
        });

        let embed = CreateEmbed::new()
            .title(format!("Logs for {}", user.name))
            .fields(fields);

        cx.interaction
            .edit_response(&cx.ctx.http, EditInteractionResponse::new().embed(embed))
            .await?;

        Ok(())
    }
}
