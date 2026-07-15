use async_trait::async_trait;
use serde::Deserialize;

use super::{EntitlementProvider, GrantData};
use crate::entitlement::service::EntitlementService;

pub struct KoFiProvider;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum KoFiType {
    Donation,
    Subscription,
    Commission,
    #[serde(rename = "Shop Order")]
    ShopOrder,
}

#[derive(Debug, Deserialize)]
pub struct KoFiPayload {
    #[serde(default)]
    pub verification_token: String,
    pub kofi_transaction_id: String,
    pub email: String,
    #[serde(rename = "type")]
    pub kind: KoFiType,
    pub is_subscription_payment: bool,
    pub is_first_subscription_payment: bool,
    pub timestamp: String,
    pub message_id: Option<String>,
}

#[async_trait]
impl EntitlementProvider for KoFiProvider {
    async fn grant(
        &self,
        service: &EntitlementService,
        data: GrantData,
    ) -> Result<(), sqlx::Error> {
        service
            .grant(data.scope, data.tier, "kofi", &data.external_id, data.expires_at)
            .await
    }

    async fn revoke(
        &self,
        service: &EntitlementService,
        external_id: &str,
    ) -> Result<(), sqlx::Error> {
        service.revoke("kofi", external_id).await
    }
}
