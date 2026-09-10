use serde_json::json;

use super::JellyfinClient;
use super::model::{AuthenticationResult, QuickConnectState};
use crate::transport::http::{ApiResult, fetch_json, send_ok};

impl JellyfinClient {
    pub async fn quick_connect_enabled(&self) -> ApiResult<bool> {
        fetch_json(super::SERVICE, "quick connect availability", || {
            self.get("QuickConnect/Enabled")
        })
        .await
    }

    pub async fn quick_connect_initiate(
        &self,
        device_id: &str,
    ) -> ApiResult<QuickConnectState> {
        fetch_json(super::SERVICE, "quick connect initiate", || {
            self.device("QuickConnect/Initiate", device_id)
        })
        .await
    }

    pub async fn quick_connect_poll(
        &self,
        secret: &str,
    ) -> ApiResult<QuickConnectState> {
        fetch_json(super::SERVICE, "quick connect poll", || {
            self.get("QuickConnect/Connect").query(&[("secret", secret)])
        })
        .await
    }

    pub async fn authenticate_with_quick_connect(
        &self,
        secret: &str,
        device_id: &str,
    ) -> ApiResult<AuthenticationResult> {
        fetch_json(super::SERVICE, "quick connect authenticate", || {
            self.device("Users/AuthenticateWithQuickConnect", device_id)
                .json(&json!({ "Secret": secret }))
        })
        .await
    }

    pub async fn revoke_device(&self, device_id: &str) -> ApiResult<()> {
        send_ok(super::SERVICE, "device revocation", || {
            self.delete("Devices").query(&[("id", device_id)])
        })
        .await
    }
}
