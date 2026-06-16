use futures::{StreamExt, stream};
use serenity::all::{
    AutoArchiveDuration,
    ChannelType,
    CreateAttachment,
    CreateEmbed,
    CreateMessage,
    CreateThread,
    Http,
    Mentionable,
    Message,
};
use sqlx::{Database, Pool};
use tracing::debug;
use zayden_core::CoreError;

use crate::{
    Result,
    TicketError,
    TicketGuildManager,
    send_support_message,
    thread_name,
};

pub struct SupportMessageCommand;

impl SupportMessageCommand {
    pub async fn run<Db: Database, GuildManager: TicketGuildManager<Db>>(
        http: &Http,
        message: &Message,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let Some(guild_id) = message.guild_id else {
            return Err(TicketError::ZaydenCore(CoreError::MissingGuildId));
        };

        let row = match GuildManager::get(pool, guild_id).await {
            Ok(Some(row)) => row,
            Ok(None) | Err(sqlx::Error::RowNotFound) => {
                debug!(%guild_id, "no ticket configuration found for guild; ignoring support message");
                return Ok(());
            },
            Err(e) => return Err(e.into()),
        };

        let Some(support_channel) = row.channel_id() else {
            return Err(TicketError::Internal(format!(
                "guild {guild_id} has no support channel configured"
            )));
        };

        let channel_id = message.channel_id.expect_channel();

        if support_channel != channel_id {
            debug!(%guild_id, %channel_id, "message not in support channel; ignoring");
            return Ok(());
        }

        let role_ids = row.role_ids();

        let thread_name = thread_name(
            row.thread_id,
            message.author.display_name(),
            &message.content,
        );

        let thread = channel_id
            .create_thread(
                http,
                CreateThread::new(thread_name)
                    .kind(ChannelType::PrivateThread)
                    .auto_archive_duration(AutoArchiveDuration::OneWeek),
            )
            .await?;

        GuildManager::update_thread_id(pool, guild_id).await?;

        let issue = CreateEmbed::new().title("Issue").description(&message.content);

        let attachments = stream::iter(message.attachments.iter())
            .filter_map(|attachment| async move {
                let bytes = attachment.download().await.ok()?;
                Some(CreateAttachment::bytes(bytes, attachment.filename.clone()))
            })
            .collect::<Vec<_>>()
            .await;

        let mentions = if role_ids.is_empty() {
            let owner_id = guild_id.to_partial_guild(http).await?.owner_id;
            vec![message.author.mention(), owner_id.mention()]
        } else {
            role_ids
                .into_iter()
                .map(|id| id.mention())
                .chain([message.author.mention()])
                .collect::<Vec<_>>()
        };

        send_support_message(http, thread.id, &mentions, vec![
            CreateMessage::new().embed(issue).files(attachments),
        ])
        .await?;

        message.delete(http, Some("Support message deleted")).await?;

        Ok(())
    }
}
