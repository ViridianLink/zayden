use async_trait::async_trait;
use jiff::{SignedDuration, Timestamp};
use serde::Deserialize;

use super::{EntitlementProvider, GrantData};
use crate::entitlement::service::EntitlementService;

pub struct KoFiProvider;

impl KoFiProvider {
    pub const SUBSCRIPTION_GRACE_DAYS: i64 = 32;

    #[must_use]
    pub fn subscription_expiry(now: Timestamp) -> Option<Timestamp> {
        now.checked_add(SignedDuration::from_hours(
            Self::SUBSCRIPTION_GRACE_DAYS * 24,
        ))
        .ok()
    }
}

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

impl KoFiPayload {
    #[must_use]
    pub fn verification_ok(&self, expected: Option<&str>) -> bool {
        matches!(expected, Some(secret) if self.verification_token == secret)
    }
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
