use std::marker::PhantomData;

use serenity::all::{GenericChannelId, Http};
use sqlx::{Database, Pool};
use zayden_core::EmojiCache;

use super::{Event, EventRow};
use crate::goals::GoalHandler;
use crate::{GoalsManager, Result};

pub struct Dispatch<'a, Db: Database, Manager: GoalsManager<Db> + Send + Sync> {
    http: &'a Http,
    pool: &'a Pool<Db>,
    emojis: &'a EmojiCache,
    _manager: PhantomData<Manager>,
}

impl<'a, Db, Manager> Dispatch<'a, Db, Manager>
where
    Db: Database,
    Manager: GoalsManager<Db> + Send + Sync,
{
    pub const fn new(
        http: &'a Http,
        pool: &'a Pool<Db>,
        emojis: &'a EmojiCache,
    ) -> Self {
        Self { http, pool, emojis, _manager: PhantomData }
    }

    pub async fn fire(
        &self,
        channel: GenericChannelId,
        row: &mut dyn EventRow,
        event: Event,
    ) -> Result<Event> {
        GoalHandler::process_goals::<Db, Manager>(
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
