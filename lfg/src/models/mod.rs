pub mod post;
pub mod timezone_manager;

use async_trait::async_trait;
pub use post::{PostBuilder, PostManager, PostRow};
use serenity::all::UserId;
use sqlx::{Database, Pool};
pub use timezone_manager::TimezoneManager;

#[async_trait]
pub trait Savable<Db: Database, T> {
    async fn save(pool: &Pool<Db>, item: T) -> sqlx::Result<Db::QueryResult>;
}

pub trait Join {
    fn fireteam_size(&self) -> i16;

    fn fireteam(&self) -> impl Iterator<Item = UserId>;

    fn fireteam_len(&self) -> i16;

    fn alternatives(&self) -> impl Iterator<Item = UserId>;

    fn is_full(&self) -> bool {
        self.fireteam_len() >= self.fireteam_size()
    }
}
