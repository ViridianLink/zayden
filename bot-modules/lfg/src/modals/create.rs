use async_trait::async_trait;
use serenity::all::{
    AutoArchiveDuration, ChannelId, Context, CreateComponent, CreateForumPost,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, DiscordJsonError,
    ErrorResponse, GenericChannelId, GuildId, HttpError, JsonErrorCode, Mentionable,
    ModalInteraction, RoleId,
};
use sqlx::prelude::FromRow;
use sqlx::{Database, Pool};
use tracing::warn;
use zayden_core::{CronJobData, parse_modal_components};

use crate::cron::create_reminders;
use crate::templates::{DefaultTemplate, Template};
use crate::{ACTIVITIES, Error, PostBuilder, PostManager, Result};
use crate::{PostRow, Savable, TimezoneManager};

use super::start_time;

#[async_trait]
pub trait GuildManager<Db: Database> {
    async fn row(pool: &Pool<Db>, id: impl Into<GuildId> + Send) -> sqlx::Result<Option<GuildRow>>;
}

#[derive(FromRow)]
pub struct GuildRow {
    pub lfg_channel_id: Option<i64>,
    pub lfg_role_id: Option<i64>,
    pub lfg_scheduled_thread_id: Option<i64>,
}

impl GuildRow {
    pub fn channel_id(&self) -> Option<ChannelId> {
        self.lfg_channel_id.map(|id| ChannelId::new(id as u64))
    }

    pub fn role_id(&self) -> Option<RoleId> {
        self.lfg_role_id.map(|id| RoleId::new(id as u64))
    }

    pub fn scheduled_channel(&self) -> Option<GenericChannelId> {
        self.lfg_scheduled_thread_id
            .map(|id| GenericChannelId::new(id as u64))
    }
}

pub struct Create;

impl Create {
    pub async fn run<
        Data: CronJobData<Db>,
        Db: Database,
        GuildHandler: GuildManager<Db>,
        PostHandler: PostManager<Db> + Savable<Db, PostRow>,
        TzManager: TimezoneManager<Db>,
    >(
        ctx: &Context,
        interaction: &ModalInteraction,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let guild_id = interaction.guild_id.ok_or(Error::MissingGuildId)?;

        let mut inputs = parse_modal_components(interaction.data.components.as_slice());

        let activity = inputs
            .remove("activity")
            .expect("Activity should exist as it's required")
            .pop()
            .expect("At least one value is required");
        let fireteam_size = inputs
            .remove("fireteam_size")
            .expect("Fireteam size should exist as it's required")
            .pop()
            .expect("At least one value is required")
            .parse::<i16>()
            .unwrap();
        let description = match inputs.remove("description") {
            Some(mut description) => description
                .pop()
                .expect("At least one value is required")
                .chars()
                .take(1024)
                .collect::<String>(),
            None => activity.to_string(),
        };
        let start_time_str = inputs
            .remove("start_time")
            .expect("Start time should exist as it's required")
            .pop()
            .expect("At least one value is required");

        let timezone = TzManager::get(pool, interaction.user.id, &interaction.locale)
            .await
            .unwrap();

        let start_time = match start_time(timezone, &start_time_str) {
            Ok(time) => time,
            Err(Error::InvalidDateTime(f)) => {
                interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .content(format!(
                                    "Bot currently only accepts {f} for dates and time."
                                ))
                                .ephemeral(true),
                        ),
                    )
                    .await?;
                return Ok(());
            }
            Err(e) => panic!("Unhandled error: {e}"),
        };

        let str_time = start_time.strftime("%d %b %H:%M %Z");

        let mut post = PostBuilder::new(
            interaction.user.id,
            activity.to_string(),
            start_time,
            description,
            fireteam_size as i16,
        );

        let embed = DefaultTemplate::thread_embed(&post, interaction.user.display_name());
        let row = DefaultTemplate::main_row();

        let lfg_guild = GuildHandler::row(pool, guild_id)
            .await
            .unwrap()
            .ok_or(Error::MissingSetup)?;

        let channel = match lfg_guild.channel_id() {
            Some(id) => id
                .to_guild_channel(&ctx.http, Some(guild_id))
                .await
                .unwrap(),
            None => return Err(Error::MissingSetup),
        };

        let tags = channel
            .available_tags
            .iter()
            .filter(|tag| {
                tag.name.to_lowercase()
                    == ACTIVITIES
                        .iter()
                        .find(|a| activity.to_lowercase().contains(&a.name.to_lowercase()))
                        .map(|a| a.category.to_string())
                        .unwrap_or_default()
                        .to_lowercase()
            })
            .map(|tag| tag.id)
            .collect::<Vec<_>>();

        let thread = match channel
            .id
            .create_forum_post(
                &ctx.http,
                CreateForumPost::new(
                    format!("{} - {}", activity, str_time),
                    CreateMessage::new()
                        .embed(embed)
                        .components(vec![CreateComponent::ActionRow(row)]),
                )
                .auto_archive_duration(AutoArchiveDuration::OneWeek)
                .set_applied_tags(tags),
            )
            .await
        {
            Ok(thread) => thread,
            Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(ErrorResponse {
                error:
                    DiscordJsonError {
                        code: JsonErrorCode::TagRequiredForForumPost,
                        ..
                    },
                ..
            }))) => {
                return Err(Error::TagRequired);
            }
            r => r.unwrap(),
        };

        let content = match lfg_guild.role_id() {
            Some(role) => format!("{} {}", role.mention(), interaction.user.mention()),
            None => interaction.user.mention().to_string(),
        };

        thread
            .send_message(&ctx.http, CreateMessage::new().content(content))
            .await
            .unwrap();

        if let Some(channel_id) = lfg_guild.scheduled_channel() {
            let embed =
                DefaultTemplate::message_embed(&post, interaction.user.display_name(), thread.id);

            match channel_id
                .send_message(&ctx.http, CreateMessage::new().embed(embed))
                .await
            {
                Ok(msg) => post = post.schedule_channel(channel_id).alt_message(msg.id),
                Err(e) => {
                    warn!("Error posting scheduled message: {e}")
                }
            }
        }

        let post = post.id(thread.id).build();

        create_reminders::<Data, Db, PostHandler>(ctx, &post).await;

        PostHandler::save(pool, post).await.unwrap();

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Successfully created post.")
                        .ephemeral(true),
                ),
            )
            .await
            .unwrap();

        Ok(())
    }
}
