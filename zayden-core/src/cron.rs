use std::fmt::Debug;
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;

use cron::Schedule;
use serenity::all::Context;
use sqlx::{Database, Pool};

pub type ActionFn<Db> =
    Arc<dyn Fn(Context, Pool<Db>) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

pub trait CronJobData<Db: Database>: Send + Sync + 'static {
    fn jobs(&self) -> &[CronJob<Db>];

    fn jobs_mut(&mut self) -> &mut Vec<CronJob<Db>>;
}

#[derive(Clone)]
pub struct CronJob<Db: Database> {
    pub id: String,
    pub schedule: Schedule,
    pub action_fn: ActionFn<Db>,
}

impl<Db: Database> CronJob<Db> {
    pub fn new(id: impl Into<String>, source: &str) -> Self {
        Self {
            id: id.into(),
            schedule: Schedule::from_str(source).unwrap(),
            action_fn: Self::action_fn(|_, _| async {}),
        }
    }

    fn action_fn<F, Fut>(f: F) -> ActionFn<Db>
    where
        F: Fn(Context, Pool<Db>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let action_closure = move |ctx, pool| {
            let future = f(ctx, pool);
            let boxed_dyn_future: Box<dyn Future<Output = ()> + Send> = Box::new(future);

            let pinned_future: Pin<Box<dyn Future<Output = ()> + Send>> =
                Box::into_pin(boxed_dyn_future);

            pinned_future
        };

        Arc::new(action_closure)
    }

    pub fn set_action<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn(Context, Pool<Db>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.action_fn = Self::action_fn(f);
        self
    }
}

impl<Db: Database> Debug for CronJob<Db> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CronJob")
            .field("id", &self.id)
            .field("schedule", &self.schedule)
            .finish()
    }
}
