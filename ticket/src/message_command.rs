use futures::{StreamExt, stream};
use serenity::all::{
    AutoArchiveDuration, ChannelType, CreateAttachment, CreateEmbed, CreateMessage, CreateThread,
    Http, Mentionable, Message,
};
use sqlx::{Database, Pool};

use crate::{Result, TicketGuildManager, send_support_message, thread_name};

pub struct SupportMessageCommand;

impl SupportMessageCommand {
    pub async fn run<Db: Database, GuildManager: TicketGuildManager<Db>>(
        http: &Http,
        message: &Message,
        pool: &Pool<Db>,
    ) -> Result<()> {
        let Some(guild_id) = message.guild_id else {
            return Ok(());
        };

        let Some(row) = GuildManager::get(pool, guild_id).await.unwrap() else {
            return Ok(());
        };

        let Some(support_channel) = row.channel_id() else {
            return Ok(());
        };

        let channel_id = message.channel_id.expect_channel();

        if support_channel != channel_id {
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
            .await
            .unwrap();

        GuildManager::update_thread_id(pool, guild_id)
            .await
            .unwrap();

        let issue = CreateEmbed::new()
            .title("Issue")
            .description(&message.content);

        let attachments = stream::iter(message.attachments.iter())
            .then(|attachment| async move {
                CreateAttachment::bytes(
                    attachment.download().await.unwrap(),
                    attachment.filename.clone(),
                )
            })
            .collect::<Vec<_>>()
            .await;

        let mentions = if role_ids.is_empty() {
            let owner_id = guild_id.to_partial_guild(http).await.unwrap().owner_id;
            vec![message.author.mention(), owner_id.mention()]
        } else {
            role_ids
                .into_iter()
                .map(|id| id.mention())
                .chain([message.author.mention()])
                .collect::<Vec<_>>()
        };

        send_support_message(
            http,
            thread.id,
            &mentions,
            vec![CreateMessage::new().embed(issue).files(attachments)],
        )
        .await
        .unwrap();

        message
            .delete(http, Some("Support message deleted"))
            .await?;

        Ok(())
    }
}
