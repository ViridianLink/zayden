use super::SeerClient;
use super::model::{MediaType, ProfileTarget, ServiceServer, ServiceServerDetails};
use crate::transport::http::{ApiResult, fetch_json};

impl SeerClient {
    pub async fn servers(&self, kind: MediaType) -> ApiResult<Vec<ServiceServer>> {
        fetch_json(super::SERVICE, "downloader list", || {
            self.get(&format!("service/{}", kind.service()))
        })
        .await
    }

    pub async fn server_details(
        &self,
        kind: MediaType,
        server_id: i32,
    ) -> ApiResult<ServiceServerDetails> {
        fetch_json(super::SERVICE, "downloader details", || {
            self.get(&format!("service/{}/{server_id}", kind.service()))
        })
        .await
    }

    pub async fn quality_profile(
        &self,
        kind: MediaType,
        name: &str,
    ) -> ApiResult<Option<ProfileTarget>> {
        let servers = self.servers(kind).await?;

        let Some(server) = servers
            .iter()
            .find(|s| s.is_default && !s.is_4k)
            .or_else(|| servers.iter().find(|s| !s.is_4k))
        else {
            return Ok(None);
        };

        let wanted = name.to_lowercase();

        Ok(self
            .server_details(kind, server.id)
            .await?
            .profiles
            .iter()
            .find(|p| p.name.to_lowercase() == wanted)
            .map(|p| ProfileTarget { server_id: server.id, profile_id: p.id }))
    }
}
