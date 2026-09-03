use std::sync::Arc;

use futures::{StreamExt, stream};
use serenity::all::{
    AutoArchiveDuration,
    ChannelType,
    CreateAttachment,
    CreateEmbed,
    CreateMessage,
    CreateThread,
    Http,
    Message,
};
use tracing::debug;
use zayden_app::state::AppState;
use zayden_core::CoreError;

use crate::{
    ISSUE_EMBED_TITLE,
    Result,
    TicketError,
    TicketGuildRow,
    TicketStores,
    send_support_message,
    support_mentions,
    thread_name,
};

pub struct SupportMessageCommand;

impl SupportMessageCommand {
    pub async fn run(
        http: &Arc<Http>,
        message: &Message,
        app: &Arc<AppState>,
    ) -> Result<()> {
        let stores = TicketStores::from_app(app);
        let pool = &app.db;

        let Some(guild_id) = message.guild_id else {
            return Err(TicketError::ZaydenCore(CoreError::MissingGuildId));
        };

        let row = match TicketGuildRow::get(stores, pool, guild_id).await {
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

        TicketGuildRow::increment_thread_id(stores.ticket, guild_id).await?;

        let issue = CreateEmbed::new()
            .title(ISSUE_EMBED_TITLE)
            .description(&message.content);

        let attachments = stream::iter(message.attachments.iter())
            .filter_map(|attachment| async move {
                let bytes = attachment.download().await.ok()?;
                Some(CreateAttachment::bytes(bytes, attachment.filename.clone()))
            })
            .collect::<Vec<_>>()
            .await;

        let owner_id = if role_ids.is_empty() {
            Some(guild_id.to_partial_guild(http).await?.owner_id)
        } else {
            None
        };
        let mentions = support_mentions(role_ids, message.author.id, owner_id);

        send_support_message(http, thread.id, &mentions, vec![
            CreateMessage::new().embed(issue).files(attachments),
        ])
        .await?;

        message.delete(http, Some("Support message deleted")).await?;

        Ok(())
    }
}
