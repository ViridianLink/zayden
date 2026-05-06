use async_trait::async_trait;
use jiff::{Span, Timestamp};
use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateScheduledEvent, EditInteractionResponse,
    Permissions, ResolvedOption, ScheduledEventType,
};
use sqlx::{PgPool, Postgres};
use zayden_core::ApplicationCommand;

use crate::{Error, Result};

pub struct Live;

impl Live {
    pub fn register() -> CreateCommand<'static> {
        CreateCommand::new("live")
            .description("Notify the server that Brad is live on Twitch")
            .default_member_permissions(Permissions::CREATE_EVENTS)
    }
}

#[async_trait]
impl ApplicationCommand<Error, Postgres> for Live {
    async fn run(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(&ctx.http).await.unwrap();

        let now = Timestamp::now();

        let in_one_min = now + Span::new().minutes(1);
        let in_seven_hours = now + Span::new().hours(7);

        interaction
            .guild_id
            .unwrap()
            .create_scheduled_event(
                &ctx.http,
                CreateScheduledEvent::new(
                    ScheduledEventType::External,
                    "Brad is LIVE",
                    serenity::all::Timestamp::from_unix_timestamp(in_one_min.as_second())
                        .expect("Timestamp should be in bounds"),
                )
                .location("https://www.twitch.tv/bradleythebradster")
                .end_time(
                    serenity::all::Timestamp::from_unix_timestamp(in_seven_hours.as_second())
                        .expect("Timestamp should be in bounds"),
                ),
            )
            .await
            .unwrap();

        interaction
            .edit_response(
                &ctx.http,
                EditInteractionResponse::new().content("Event successfully created."),
            )
            .await
            .unwrap();

        Ok(())
    }

    fn command(&self) -> CreateCommand<'_> {
        CreateCommand::new("live").description("Notify the server that Brad is live on Twitch")
    }
}
