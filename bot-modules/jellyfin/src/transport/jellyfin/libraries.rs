use serde_json::json;

use super::JellyfinClient;
use super::model::VirtualFolder;
use crate::transport::http::{ApiResult, fetch_json, send_ok};

impl JellyfinClient {
    pub async fn virtual_folders(&self) -> ApiResult<Vec<VirtualFolder>> {
        fetch_json(super::SERVICE, "virtual folder list", || {
            self.get("Library/VirtualFolders")
        })
        .await
    }

    pub async fn create_virtual_folder(
        &self,
        name: &str,
        collection_type: &str,
        path: &str,
    ) -> ApiResult<()> {
        send_ok(super::SERVICE, "virtual folder creation", || {
            self.post("Library/VirtualFolders")
                .query(&[
                    ("name", name),
                    ("collectionType", collection_type),
                    ("refreshLibrary", "true"),
                ])
                .json(&json!({
                    "LibraryOptions": { "PathInfos": [{ "Path": path }] }
                }))
        })
        .await
    }

    pub async fn delete_virtual_folder(&self, name: &str) -> ApiResult<()> {
        match send_ok(super::SERVICE, "virtual folder deletion", || {
            self.delete("Library/VirtualFolders")
                .query(&[("name", name), ("refreshLibrary", "false")])
        })
        .await
        {
            Err(e) if e.is_not_found() => Ok(()),
            other => other,
        }
    }

    pub async fn virtual_folder_id(&self, name: &str) -> ApiResult<Option<String>> {
        Ok(self
            .virtual_folders()
            .await?
            .into_iter()
            .find(|f| f.name == name)
            .and_then(|f| f.item_id))
    }
}
