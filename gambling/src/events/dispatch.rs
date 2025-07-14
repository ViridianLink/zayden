use std::marker::PhantomData;

use serenity::all::{GenericChannelId, Http};
use sqlx::{Database, Pool};

use crate::GoalsManager;
use crate::goals::GoalHandler;

use super::{Event, EventRow};

pub struct Dispatch<'a, Db: Database, Manager: GoalsManager<Db>> {
    http: &'a Http,
    pool: &'a Pool<Db>,
    _manager: PhantomData<Manager>,
}

impl<'a, Db, Manager> Dispatch<'a, Db, Manager>
where
    Db: Database,
    Manager: GoalsManager<Db>,
{
    pub fn new(http: &'a Http, pool: &'a Pool<Db>) -> Self {
        Self {
            http,
            pool,
            _manager: PhantomData,
        }
    }

    pub async fn fire(
        &self,
        channel: GenericChannelId,
        row: &mut dyn EventRow,
        event: Event,
    ) -> sqlx::Result<Event> {
        GoalHandler::process_goals::<Db, Manager>(self.http, self.pool, channel, row, event).await
    }
}
