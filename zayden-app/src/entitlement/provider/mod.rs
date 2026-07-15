mod discord;
mod kofi;

use async_trait::async_trait;
pub use discord::DiscordProvider;
use jiff::Timestamp;
pub use kofi::{KoFiPayload, KoFiProvider, KoFiType};

use super::service::EntitlementService;
use super::types::{EntitlementScope, Tier};

#[async_trait]
pub trait EntitlementProvider: Send + Sync {
    async fn grant(
        &self,
        service: &EntitlementService,
        data: GrantData,
    ) -> Result<(), sqlx::Error>;

    async fn revoke(
        &self,
        service: &EntitlementService,
        external_id: &str,
    ) -> Result<(), sqlx::Error>;
}

pub struct GrantData {
    pub external_id: String,
    pub scope: EntitlementScope,
    pub tier: Tier,
    pub expires_at: Option<Timestamp>,
}
