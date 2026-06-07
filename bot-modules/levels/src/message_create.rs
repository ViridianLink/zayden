use jiff::{Span, Timestamp};
use serenity::all::Message;
use sqlx::{Database, Pool};

use super::LevelsRow;
use crate::{FullLevelRow, LevelsManager};

pub async fn message_create<Db: Database, Manager: LevelsManager<Db>>(
    message: &Message,
    pool: &Pool<Db>,
) -> Result<Option<i32>, sqlx::Error> {
    let Some(_guild_id) = message.guild_id else {
        return Ok(None);
    };

    let mut row = Manager::full_row(pool, message.author.id)
        .await?
        .unwrap_or_else(|| FullLevelRow::new(message.author.id));

    let xp_cooldown =
        row.last_xp().checked_add(Span::new().minutes(1)).unwrap_or(Timestamp::MAX);

    if xp_cooldown > Timestamp::now() {
        return Ok(None);
    }

    let new_level = row.new_message();

    Manager::save(pool, row).await?;

    Ok(new_level)
}
