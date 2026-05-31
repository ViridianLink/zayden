use jiff::{Span, Timestamp};
use serenity::all::Message;
use sqlx::{Database, Pool};

use super::LevelsRow;
use crate::{FullLevelRow, LevelsManager};

pub async fn message_create<Db: Database, Manager: LevelsManager<Db>>(
    message: &Message,
    pool: &Pool<Db>,
) -> Option<i32> {
    message.guild_id?;

    let mut row = Manager::full_row(pool, message.author.id)
        .await
        .expect("DB query")
        .unwrap_or_else(|| FullLevelRow::new(message.author.id));

    let xp_cooldown = row
        .last_xp()
        .checked_add(Span::new().minutes(1))
        .expect("Timestamp should be within legal range");

    if xp_cooldown > Timestamp::now() {
        return None;
    }

    let new_level = row.new_message();

    Manager::save(pool, row).await.expect("failed to save xp row");

    new_level
}
