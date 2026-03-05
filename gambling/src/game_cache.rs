use std::collections::HashMap;
use std::sync::Arc;

use jiff::{Span, Timestamp};
use serenity::all::UserId;
use tokio::sync::RwLock;

use crate::ctx_data::GamblingData;
use crate::{Error, Result};

#[derive(Default)]
pub struct GameCache(HashMap<UserId, Timestamp>);

impl GameCache {
    pub async fn can_play<D: GamblingData>(
        data: Arc<RwLock<D>>,
        id: impl Into<UserId>,
    ) -> Result<()> {
        let id = id.into();

        let data = data.read().await;

        if let Some(last_played) = data.game_cache().0.get(&id) {
            let cooldown_over = *last_played + Span::new().seconds(5);

            if cooldown_over >= Timestamp::now() {
                return Err(Error::Cooldown(cooldown_over.as_second()));
            }
        }

        Ok(())
    }

    pub async fn update<D: GamblingData>(data: Arc<RwLock<D>>, id: impl Into<UserId>) {
        let mut data = data.write().await;

        let cache = data.game_cache_mut();

        cache.0.insert(id.into(), Timestamp::now());
    }
}
