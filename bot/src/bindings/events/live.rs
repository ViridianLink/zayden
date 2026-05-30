use std::borrow::Cow;

use async_trait::async_trait;
use jiff::{Span, Timestamp};
use serenity::all::{
    CreateCommand,
    CreateScheduledEvent,
    EditInteractionResponse,
    GuildId,
    Permissions,
    ScheduledEventType,
};
use zayden_core::{CommandScope, Error, HandlerError, InvocationCtx, ModuleCommand};

static LIVE_GUILDS: [GuildId; 1] = [crate::BRADSTER_GUILD];

pub(super) struct Live;

impl Live {
    pub(super) fn register() -> CreateCommand<'static> {
        CreateCommand::new("live")
            .description("Notify the server that Brad is live on Twitch")
            .default_member_permissions(Permissions::CREATE_EVENTS)
    }
}

#[async_trait]
impl ModuleCommand for Live {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed("live")
    }

    fn definition(&self) -> CreateCommand<'static> {
        Self::register()
    }

    fn scope(&self) -> CommandScope {
        CommandScope::Guilds(Cow::Borrowed(&LIVE_GUILDS))
    }

    async fn run(&self, cx: &InvocationCtx<'_>) -> Result<(), HandlerError> {
        cx.interaction.defer(&cx.ctx.http).await.map_err(HandlerError::new)?;

        let guild_id = cx
            .interaction
            .guild_id
            .ok_or_else(|| HandlerError::from_respond(Error::MissingGuildId))?;

        let now = Timestamp::now();
        let in_one_min = now + Span::new().minutes(1);
        let in_seven_hours = now + Span::new().hours(7);

        guild_id
            .create_scheduled_event(
                &cx.ctx.http,
                CreateScheduledEvent::new(
                    ScheduledEventType::External,
                    "Brad is LIVE",
                    serenity::all::Timestamp::from_unix_timestamp(
                        in_one_min.as_second(),
                    )
                    .expect("timestamp must be in bounds"),
                )
                .location("https://www.twitch.tv/bradleythebradster")
                .end_time(
                    serenity::all::Timestamp::from_unix_timestamp(
                        in_seven_hours.as_second(),
                    )
                    .expect("timestamp must be in bounds"),
                ),
            )
            .await
            .map_err(HandlerError::new)?;

        cx.interaction
            .edit_response(
                &cx.ctx.http,
                EditInteractionResponse::new()
                    .content("Event successfully created."),
            )
            .await
            .map_err(HandlerError::new)?;

        Ok(())
    }
}
