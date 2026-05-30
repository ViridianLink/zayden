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

        let cooldown_over = {
            let data = data.read().await;
            data.game_cache()
                .0
                .get(&id)
                .map(|last_played| *last_played + Span::new().seconds(5))
        };

        if cooldown_over.is_some_and(|co| co >= Timestamp::now()) {
            return Err(Error::Cooldown(
                cooldown_over.expect("checked above").as_second(),
            ));
        }

        Ok(())
    }

    pub async fn update<D: GamblingData>(
        data: Arc<RwLock<D>>,
        id: impl Into<UserId>,
    ) {
        let id = id.into();
        let now = Timestamp::now();
        data.write().await.game_cache_mut().0.insert(id, now);
    }
}
