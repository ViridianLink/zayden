use serenity::all::{
    Context,
    CreateInteractionResponse,
    DiscordJsonError,
    EditMessage,
    EditThread,
    ErrorResponse,
    HttpError,
    JsonErrorCode,
    ModalInteraction,
};
use sqlx::{Database, Pool};
use zayden_core::{CronJobData, parse_modal_components};

use super::start_time;
use crate::cron::create_reminders;
use crate::templates::DefaultTemplate;
use crate::utils::update_embeds;
use crate::{
    LfgError,
    PostBuilder,
    PostManager,
    PostRow,
    Result,
    Savable,
    TimezoneManager,
};

pub struct Edit;

impl Edit {
    pub async fn run<
        Data: CronJobData<Db>,
        Db: Database,
        Manager: PostManager<Db> + Savable<Db, PostRow>,
        TzManager: TimezoneManager<Db>,
    >(
        ctx: &Context,
        interaction: &ModalInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        interaction
            .create_response(&ctx.http, CreateInteractionResponse::Acknowledge)
            .await?;

        let mut inputs =
            parse_modal_components(interaction.data.components.as_slice());

        let activity =
            inputs.remove("activity").and_then(|mut v| v.pop()).unwrap_or_default();

        let fireteam_size_str = inputs
            .remove("fireteam_size")
            .and_then(|mut v| v.pop())
            .unwrap_or_default();
        let fireteam_size = fireteam_size_str
            .parse::<i16>()
            .map_err(|_e| LfgError::InvalidFireteamSize)?;

        let description = inputs.remove("description").map_or_else(
            || activity.to_string(),
            |mut d| d.pop().unwrap_or_default().to_string(),
        );

        let start_time_str = inputs
            .remove("start_time")
            .and_then(|mut v| v.pop())
            .unwrap_or_default();

        let timezone =
            TzManager::get(pool, interaction.user.id, &interaction.locale).await?;

        let start_time = start_time(timezone, &start_time_str)?;

        let str_time = start_time.strftime("%d %b %H:%M %Z");

        let post = PostBuilder::from(
            Manager::post_row(pool, interaction.channel_id).await?,
        )
        .activity(activity.to_string())
        .fireteam_size(fireteam_size)
        .description(description)
        .start(start_time)
        .build();

        Manager::edit(pool, &post).await?;

        let thread = interaction.channel_id.expect_thread();

        match thread
            .edit(
                &ctx.http,
                EditThread::new().name(format!("{activity} - {str_time}")),
            )
            .await
        {
            Ok(_) => {},
            Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
                ErrorResponse {
                    error:
                        DiscordJsonError { code: JsonErrorCode::UnknownChannel, .. },
                    ..
                },
            ))) => {
                // Thread/Event deleted
                debug!();
                return Ok(());
            },
            Err(e) => return Err(e.into()),
        }

        let embed = update_embeds::<DefaultTemplate>(
            &ctx.http,
            &post,
            interaction.user.display_name(),
            thread,
        )
        .await?;

        create_reminders::<Data, Db, Manager>(ctx, &post).await;

        thread
            .widen()
            .edit_message(
                &ctx.http,
                thread.get().into(),
                EditMessage::new().embed(embed),
            )
            .await?;

        Ok(())
    }
}
