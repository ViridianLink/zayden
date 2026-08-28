use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use tokio::sync::{Mutex as AsyncMutex, MutexGuard};
use twilight_model::id::Id;
use twilight_model::id::marker::GuildMarker;

type Key = (Id<GuildMarker>, &'static str);

struct Slot {
    latest: u64,
    gate: Arc<AsyncMutex<()>>,
}

static SLOTS: LazyLock<Mutex<HashMap<Key, Slot>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) struct Claim {
    key: Key,
    ticket: u64,
    gate: Arc<AsyncMutex<()>>,
}

pub(crate) fn claim(guild_id: Id<GuildMarker>, module: &'static str) -> Claim {
    let key = (guild_id, module);

    let mut slots = SLOTS.lock().unwrap_or_else(|e| e.into_inner());
    let slot = slots
        .entry(key)
        .or_insert_with(|| Slot { latest: 0, gate: Arc::new(AsyncMutex::new(())) });

    slot.latest = slot.latest.wrapping_add(1);

    Claim { key, ticket: slot.latest, gate: Arc::clone(&slot.gate) }
}

impl Claim {
    pub(crate) async fn wait_for_turn(&self) -> MutexGuard<'_, ()> {
        self.gate.lock().await
    }

    pub(crate) fn superseded(&self) -> bool {
        let slots = SLOTS.lock().unwrap_or_else(|e| e.into_inner());
        slots.get(&self.key).is_some_and(|slot| slot.latest != self.ticket)
    }
}

impl Drop for Claim {
    fn drop(&mut self) {
        let mut slots = SLOTS.lock().unwrap_or_else(|e| e.into_inner());

        if slots.get(&self.key).is_some_and(|slot| slot.latest == self.ticket) {
            slots.remove(&self.key);
        }
    }
}
