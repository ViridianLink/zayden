use serenity::all::Message;
use sqlx::PgPool;

use crate::{FullLevelRow, GuildLevelRow, MessageXp};

pub async fn message_create(
    message: &Message,
    pool: &PgPool,
) -> Result<Option<i32>, sqlx::Error> {
    let Some(guild_id) = message.guild_id else {
        return Ok(None);
    };

    let author_id = message.author.id;

    let global_level =
        FullLevelRow::accrue_message(pool, author_id, MessageXp::roll()).await?;

    GuildLevelRow::accrue_message(pool, guild_id, author_id, MessageXp::roll())
        .await?;

    Ok(global_level)
}
