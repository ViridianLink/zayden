use serde_json::Value;

use super::JellyfinClient;
use crate::transport::http::{ApiResult, fetch_json, send_ok};

impl JellyfinClient {
    pub async fn plugin_configuration(&self, plugin_id: &str) -> ApiResult<Value> {
        fetch_json(super::SERVICE, "plugin configuration", || {
            self.get(&format!("Plugins/{plugin_id}/Configuration"))
        })
        .await
    }

    pub async fn set_plugin_configuration(
        &self,
        plugin_id: &str,
        configuration: &Value,
    ) -> ApiResult<()> {
        send_ok(super::SERVICE, "plugin configuration update", || {
            self.post(&format!("Plugins/{plugin_id}/Configuration"))
                .json(configuration)
        })
        .await
    }
}
