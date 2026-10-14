use serenity::all::{AutoArchiveDuration, CreateThread, Http, Message, MessageType};
use sqlx::PgPool;
use tracing::debug;

use crate::layout::thread_name;
use crate::{Result, Suggestions, SuggestionsGuildRow};

impl Suggestions {
    pub async fn message(
        http: &Http,
        message: &Message,
        pool: &PgPool,
    ) -> Result<()> {
        let Some(guild_id) = message.guild_id else {
            return Ok(());
        };

        if message.author.bot()
            || !matches!(
                message.kind,
                MessageType::Regular | MessageType::InlineReply
            )
        {
            return Ok(());
        }

        let Some(configured) = SuggestionsGuildRow::get(pool, guild_id)
            .await?
            .and_then(|row| row.channel_id())
        else {
            return Ok(());
        };

        if message.channel_id.get() != configured.get() {
            return Ok(());
        }

        debug!(%guild_id, message_id = %message.id, "opening suggestion thread");

        configured
            .create_thread_from_message(
                http,
                message.id,
                CreateThread::new(thread_name(
                    message.author.display_name(),
                    &message.content,
                ))
                .auto_archive_duration(AutoArchiveDuration::OneWeek),
            )
            .await?;

        message.react(http, '👍').await?;
        message.react(http, '👎').await?;

        Ok(())
    }
}
