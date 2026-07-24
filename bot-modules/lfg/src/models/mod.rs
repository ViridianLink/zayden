pub mod post;
pub mod timezone_manager;

pub use post::{PostBuilder, PostRow};
use serenity::all::UserId;
pub use timezone_manager::UserSettings;

pub trait Join {
    fn fireteam_size(&self) -> i16;

    fn fireteam(&self) -> impl Iterator<Item = UserId>;

    fn fireteam_len(&self) -> i16;

    fn alternatives(&self) -> impl Iterator<Item = UserId>;

    fn is_full(&self) -> bool {
        self.fireteam_len() >= self.fireteam_size()
    }
}
