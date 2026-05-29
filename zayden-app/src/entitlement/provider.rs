use async_trait::async_trait;
use serde::Deserialize;
use tracing::warn;

use super::{
    service::EntitlementService,
    types::{EntitlementScope, Tier},
};

/// Abstraction over an external billing/subscription provider that writes
/// entitlements into the shared `entitlements` table via [`EntitlementService`].
#[async_trait]
pub trait EntitlementProvider: Send + Sync {
    /// Record a new or renewed entitlement originating from this provider.
    async fn grant(&self, service: &EntitlementService, data: GrantData)
    -> Result<(), sqlx::Error>;

    /// Remove an entitlement by its provider-specific reference.
    async fn revoke(
        &self,
        service: &EntitlementService,
        external_id: &str,
    ) -> Result<(), sqlx::Error>;
}

/// Provider-neutral grant payload passed to [`EntitlementProvider::grant`].
pub struct GrantData {
    /// Provider-specific unique identifier (Discord entitlement ID, KoFi transaction ID, …).
    pub external_id: String,
    /// The principal receiving the entitlement.
    pub scope: EntitlementScope,
    /// The tier being granted.
    pub tier: Tier,
    /// Optional expiry; `None` means the entitlement does not auto-expire.
    pub expires_at: Option<jiff::Timestamp>,
}

// ── Discord provider ──────────────────────────────────────────────────────────

/// Handles Discord App Subscription entitlements.
///
/// The bot calls this provider when it receives `EntitlementCreate` /
/// `EntitlementUpdate` / `EntitlementDelete` gateway events.  Raw Discord field
/// values (IDs as `u64`, timestamps as `Option<i64>` Unix seconds) are used so
/// that this code has no Serenity dependency.
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
    /// Build a [`GrantData`] from the raw fields extracted from a Discord `Entitlement` object.
    ///
    /// * `entitlement_id`  — the Discord entitlement's `id` as a `u64`.
    /// * `user_id`         — `user_id` field from the entitlement (may be `None` for guild subs).
    /// * `guild_id`        — `guild_id` field from the entitlement (may be `None` for user subs).
    /// * `ends_at_unix`    — `ends_at` epoch seconds, `None` if the subscription doesn't expire.
    ///
    /// Returns `None` when neither `user_id` nor `guild_id` is set (unexpected from Discord).
    pub fn build_grant(
        entitlement_id: u64,
        user_id: Option<u64>,
        guild_id: Option<u64>,
        ends_at_unix: Option<i64>,
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
            }
        };

        let expires_at = ends_at_unix.and_then(|ts| {
            jiff::Timestamp::from_second(ts)
                .map_err(|e| warn!(%e, "invalid ends_at timestamp"))
                .ok()
        });

        Some(GrantData {
            external_id: entitlement_id.to_string(),
            scope,
            tier: Tier::Pro,
            expires_at,
        })
    }
}

// ── KoFi provider ─────────────────────────────────────────────────────────────

/// Handles Ko-fi donation / subscription webhooks.
///
/// Ko-fi POSTs a `application/x-www-form-urlencoded` body with a single `data`
/// field containing a JSON-encoded payload.
pub struct KoFiProvider;

/// Subset of the Ko-fi webhook payload we care about.
#[derive(Debug, Deserialize)]
pub struct KoFiPayload {
    pub kofi_transaction_id: String,
    pub email: String,
    #[serde(rename = "type")]
    pub kind: String, // "Donation", "Subscription", "Shop Order"
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
            .grant(
                data.scope,
                data.tier,
                "kofi",
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
        service.revoke("kofi", external_id).await
    }
}
