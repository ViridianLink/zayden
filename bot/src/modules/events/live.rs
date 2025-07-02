use async_trait::async_trait;
use chrono::{Duration, Utc};
use serenity::all::{
    CommandInteraction, Context, CreateCommand, CreateScheduledEvent, EditInteractionResponse,
    Permissions, ResolvedOption, ScheduledEventType,
};
use sqlx::{PgPool, Postgres};
use zayden_core::SlashCommand;

use crate::{Error, Result};

pub struct Live;

#[async_trait]
impl SlashCommand<Error, Postgres> for Live {
    async fn run(
        ctx: &Context,
        interaction: &CommandInteraction,
        _options: Vec<ResolvedOption<'_>>,
        _pool: &PgPool,
    ) -> Result<()> {
        interaction.defer(ctx).await.unwrap();

        let now = Utc::now();

        interaction
            .guild_id
            .unwrap()
            .create_scheduled_event(
                ctx,
                CreateScheduledEvent::new(
                    ScheduledEventType::External,
                    "Brad is LIVE",
                    now + Duration::minutes(1),
                )
                .location("https://www.twitch.tv/bradleythebradster")
                .end_time(now + Duration::hours(7)),
            )
            .await
            .unwrap();

        interaction
            .edit_response(
                ctx,
                EditInteractionResponse::new().content("Event successfully created."),
            )
            .await
            .unwrap();

        Ok(())
    }

    fn register(_ctx: &Context) -> Result<CreateCommand> {
        let cmd = CreateCommand::new("live")
            .description("Notify the server that Brad is live on Twitch")
            .default_member_permissions(Permissions::CREATE_EVENTS);

        Ok(cmd)
    }
}
