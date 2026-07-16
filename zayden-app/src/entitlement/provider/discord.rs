use std::collections::HashMap;

use async_trait::async_trait;
use jiff::Timestamp;
use tracing::warn;

use super::{EntitlementProvider, GrantData};
use crate::entitlement::service::EntitlementService;
use crate::entitlement::types::{EntitlementScope, Tier};

pub struct DiscordProvider;

#[async_trait]
impl EntitlementProvider for DiscordProvider {
    async fn grant(
        &self,
        service: &EntitlementService,
        data: GrantData,
    ) -> Result<(), sqlx::Error> {
        service
            .grant(
                data.scope,
                data.tier,
                "discord",
                &data.external_id,
                data.expires_at,
            )
            .await
    }

    async fn revoke(
        &self,
        service: &EntitlementService,
        external_id: &str,
    ) -> Result<(), sqlx::Error> {
        service.revoke("discord", external_id).await
    }
}

impl DiscordProvider {
    pub fn build_grant(
        entitlement_id: u64,
        user_id: Option<u64>,
        guild_id: Option<u64>,
        ends_at_unix: Option<i64>,
        sku_id: Option<u64>,
        sku_tiers: &HashMap<u64, Tier>,
    ) -> Option<GrantData> {
        let scope = match (user_id, guild_id) {
            (Some(uid), Some(gid)) => EntitlementScope::UserInGuild(uid, gid),
            (Some(uid), None) => EntitlementScope::User(uid),
            (None, Some(gid)) => EntitlementScope::Guild(gid),
            (None, None) => {
                warn!(
                    entitlement_id,
                    "Discord entitlement has neither user_id nor guild_id; skipping"
                );
                return None;
            },
        };

        let expires_at = ends_at_unix.and_then(|ts| {
            Timestamp::from_second(ts)
                .map_err(|e| warn!(%e, "invalid ends_at timestamp"))
                .ok()
        });

        let tier = sku_id
            .and_then(|sku| sku_tiers.get(&sku).copied())
            .unwrap_or_else(|| {
                warn!(
                    entitlement_id,
                    ?sku_id,
                    "Discord entitlement SKU not mapped to a tier; defaulting to Pro"
                );
                Tier::Pro
            });

        Some(GrantData {
            external_id: entitlement_id.to_string(),
            scope,
            tier,
            expires_at,
        })
    }
}
