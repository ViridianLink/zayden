use std::sync::Arc;

use bungie_api::BungieClient;

pub trait BungieClientData: Send + Sync + 'static {
    fn bungie_client(&self) -> Arc<BungieClient>;
}

impl BungieClientData for Arc<BungieClient> {
    fn bungie_client(&self) -> Self {
        Self::clone(self)
    }
}
