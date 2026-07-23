use jiff::{Span, Timestamp};
use serenity::all::Message;
use sqlx::PgPool;

use super::LevelsRow;
use crate::FullLevelRow;

pub async fn message_create(
    message: &Message,
    pool: &PgPool,
) -> Result<Option<i32>, sqlx::Error> {
    let Some(_guild_id) = message.guild_id else {
        return Ok(None);
    };

    let mut row = FullLevelRow::get(pool, message.author.id)
        .await?
        .unwrap_or_else(|| FullLevelRow::new(message.author.id));

    let xp_cooldown =
        row.last_xp().checked_add(Span::new().minutes(1)).unwrap_or(Timestamp::MAX);

    if xp_cooldown > Timestamp::now() {
        return Ok(None);
    }

    let new_level = row.new_message();

    row.save(pool).await?;

    Ok(new_level)
}
