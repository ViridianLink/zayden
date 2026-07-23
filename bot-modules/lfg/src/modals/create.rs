use async_trait::async_trait;
use serenity::all::{
    AutoArchiveDuration,
    ChannelId,
    Context,
    CreateComponent,
    CreateForumPost,
    CreateInteractionResponse,
    CreateInteractionResponseMessage,
    CreateMessage,
    DiscordJsonError,
    ErrorResponse,
    GenericChannelId,
    GuildId,
    HttpError,
    JsonErrorCode,
    Mentionable,
    ModalInteraction,
    RoleId,
};
use sqlx::prelude::FromRow;
use sqlx::{Database, Pool};
use tracing::warn;
use zayden_core::{CronJobData, as_u64, parse_modal_components};

use super::start_time;
use crate::cron::create_reminders;
use crate::templates::{DefaultTemplate, Template};
use crate::{
    ACTIVITIES,
    LfgError,
    PostBuilder,
    PostManager,
    PostRow,
    Result,
    Savable,
    TimezoneManager,
};

#[async_trait]
pub trait GuildManager<Db: Database> {
    async fn row(pool: &Pool<Db>, id: GuildId) -> sqlx::Result<Option<GuildRow>>;
}

#[derive(FromRow)]
pub struct GuildRow {
    pub lfg_channel_id: Option<i64>,
    pub lfg_role_id: Option<i64>,
    pub lfg_scheduled_thread_id: Option<i64>,
}

impl GuildRow {
    #[must_use]
    pub fn channel_id(&self) -> Option<ChannelId> {
        self.lfg_channel_id.map(|id| ChannelId::new(as_u64(id)))
    }

    #[must_use]
    pub fn role_id(&self) -> Option<RoleId> {
        self.lfg_role_id.map(|id| RoleId::new(as_u64(id)))
    }

    #[must_use]
    pub fn scheduled_channel(&self) -> Option<GenericChannelId> {
        self.lfg_scheduled_thread_id.map(|id| GenericChannelId::new(as_u64(id)))
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
        let guild_id = interaction.guild_id.ok_or(LfgError::MissingGuildId)?;

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
            |mut d| {
                d.pop().unwrap_or_default().chars().take(1024).collect::<String>()
            },
        );

        let start_time_str = inputs
            .remove("start_time")
            .and_then(|mut v| v.pop())
            .unwrap_or_default();

        let timezone =
            TzManager::get(pool, interaction.user.id, &interaction.locale).await?;

        let start_time = start_time(timezone, &start_time_str)?;

        let str_time = start_time.strftime("%d %b %H:%M %Z");

        let mut post = PostBuilder::new(
            interaction.user.id,
            activity.to_string(),
            start_time,
            description,
            fireteam_size,
        );

        let embed =
            DefaultTemplate::thread_embed(&post, interaction.user.display_name());
        let row = DefaultTemplate::main_row();

        let lfg_guild = GuildHandler::row(pool, guild_id)
            .await?
            .ok_or(LfgError::MissingSetup)?;

        let channel = match lfg_guild.channel_id() {
            Some(id) => id.to_guild_channel(&ctx.http, Some(guild_id)).await?,
            None => return Err(LfgError::MissingSetup),
        };

        let tags = channel
            .available_tags
            .iter()
            .filter(|tag| {
                tag.name.to_lowercase()
                    == ACTIVITIES
                        .iter()
                        .find(|a| {
                            activity.to_lowercase().contains(&a.name.to_lowercase())
                        })
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
                    format!("{activity} - {str_time}"),
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
            Err(serenity::Error::Http(HttpError::UnsuccessfulRequest(
                ErrorResponse {
                    error:
                        DiscordJsonError {
                            code: JsonErrorCode::TagRequiredForForumPost,
                            ..
                        },
                    ..
                },
            ))) => {
                return Err(LfgError::TagRequired);
            },
            r => r?,
        };

        let content = lfg_guild.role_id().map_or_else(
            || interaction.user.mention().to_string(),
            |role| format!("{} {}", role.mention(), interaction.user.mention()),
        );

        thread
            .send_message(&ctx.http, CreateMessage::new().content(content))
            .await?;

        if let Some(channel_id) = lfg_guild.scheduled_channel() {
            let embed = DefaultTemplate::message_embed(
                &post,
                interaction.user.display_name(),
                thread.id,
            );

            match channel_id
                .send_message(&ctx.http, CreateMessage::new().embed(embed))
                .await
            {
                Ok(msg) => {
                    post = post.schedule_channel(channel_id).alt_message(msg.id);
                },
                Err(e) => {
                    warn!("Error posting scheduled message: {e}");
                },
            }
        }

        let post = post.id(thread.id).build();

        create_reminders::<Data, Db, PostHandler>(ctx, &post).await;

        PostHandler::save(pool, post).await?;

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new()
                        .content("Successfully created post.")
                        .ephemeral(true),
                ),
            )
            .await?;

        Ok(())
    }
}
