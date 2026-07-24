use serenity::all::{GenericChannelId, Http};
use sqlx::PgPool;
use zayden_core::EmojiCache;

use super::{Event, EventRow};
use crate::Result;
use crate::goals::GoalHandler;

pub struct Dispatch<'a> {
    http: &'a Http,
    pool: &'a PgPool,
    emojis: &'a EmojiCache,
}

impl<'a> Dispatch<'a> {
    pub const fn new(
        http: &'a Http,
        pool: &'a PgPool,
        emojis: &'a EmojiCache,
    ) -> Self {
        Self { http, pool, emojis }
    }

    pub async fn fire(
        &self,
        channel: GenericChannelId,
        row: &mut dyn EventRow,
        event: Event,
    ) -> Result<Event> {
        GoalHandler::process_goals(
            self.http,
            self.pool,
            self.emojis,
            channel,
            row,
            event,
        )
        .await
    }
}
