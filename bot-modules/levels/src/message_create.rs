use jiff::{Span, Timestamp};
use serenity::all::Message;
use sqlx::PgPool;

use super::LevelsRow;
use crate::{FullLevelRow, GuildLevelRow};

fn on_cooldown(last_xp: Timestamp) -> bool {
    let cooldown =
        last_xp.checked_add(Span::new().minutes(1)).unwrap_or(Timestamp::MAX);
    cooldown > Timestamp::now()
}

pub async fn message_create(
    message: &Message,
    pool: &PgPool,
) -> Result<Option<i32>, sqlx::Error> {
    let Some(guild_id) = message.guild_id else {
        return Ok(None);
    };

    let mut global = FullLevelRow::get(pool, message.author.id)
        .await?
        .unwrap_or_else(|| FullLevelRow::new(message.author.id));

    let global_level = if on_cooldown(global.last_xp()) {
        None
    } else {
        let new_level = global.new_message();
        global.save(pool).await?;
        new_level
    };

    let mut guild = GuildLevelRow::get(pool, guild_id, message.author.id)
        .await?
        .unwrap_or_else(|| GuildLevelRow::new(guild_id, message.author.id));

    if !on_cooldown(guild.last_xp()) {
        guild.new_message();
        guild.save(pool).await?;
    }

    Ok(global_level)
}
