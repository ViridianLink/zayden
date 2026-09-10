use std::sync::Arc;

use tracing::warn;

use crate::error::Result;
use crate::guest::naming;
use crate::runtime::JellyfinRuntime;

pub async fn delete_guest(
    runtime: &Arc<JellyfinRuntime>,
    jellyfin_user_id: &str,
) -> Result<()> {
    runtime.jellyfin.delete_user(jellyfin_user_id).await?;
    Ok(())
}

pub async fn delete_library(
    runtime: &Arc<JellyfinRuntime>,
    party_id: i64,
) -> Result<()> {
    runtime.jellyfin.delete_virtual_folder(&naming::library_name(party_id)).await?;
    Ok(())
}

#[derive(Debug, Clone, Default)]
pub struct ManagedObjects {
    pub guests: Vec<(String, i64)>,
    pub libraries: Vec<(String, i64)>,
}

pub async fn list_managed(runtime: &Arc<JellyfinRuntime>) -> Result<ManagedObjects> {
    let mut found = ManagedObjects::default();

    for user in runtime.jellyfin.users().await? {
        if !naming::is_managed_guest(&user.name) {
            continue;
        }
        match naming::party_id_of_guest(&user.name) {
            Some(party_id) => found.guests.push((user.id, party_id)),
            None => warn!(name = %user.name, "managed guest with no party id"),
        }
    }

    for folder in runtime.jellyfin.virtual_folders().await? {
        if !naming::is_managed_library(&folder.name) {
            continue;
        }
        if let Some(party_id) = naming::party_id_of_library(&folder.name) {
            found.libraries.push((folder.name, party_id));
        }
    }

    Ok(found)
}
