use crate::GameCache;

pub trait GamblingData: Send + Sync + 'static {
    fn game_cache(&self) -> &GameCache;
}
