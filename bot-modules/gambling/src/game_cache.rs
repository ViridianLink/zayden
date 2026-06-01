use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use jiff::{Span, Timestamp};
use serenity::all::UserId;

use crate::{GamblingError, Result};

#[derive(Default)]
pub struct GameCache(DashMap<UserId, Timestamp>);

impl GameCache {
    pub fn check_and_set(&self, id: UserId) -> Result<()> {
        let now = Timestamp::now();

        match self.0.entry(id) {
            Entry::Vacant(e) => {
                e.insert(now);
            },
            Entry::Occupied(mut e) => {
                let cooldown_over = *e.get() + Span::new().seconds(5);
                if cooldown_over >= now {
                    return Err(GamblingError::Cooldown(cooldown_over.as_second()));
                }
                *e.get_mut() = now;
            },
        }

        Ok(())
    }
}
