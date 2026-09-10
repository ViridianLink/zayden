use std::sync::Arc;

use crate::error::Result;
use crate::guest::naming;
use crate::runtime::JellyfinRuntime;
use crate::transport::jellyfin::model::UserPolicy;

#[must_use]
pub fn generate_password() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};

    // Two independently-seeded hashers give 128 bits of OS-seeded entropy
    // without pulling `rand` into this crate for one call site.
    let a = RandomState::new().build_hasher().finish();
    let b = RandomState::new().build_hasher().finish();
    format!("{a:016x}{b:016x}")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvisionedGuest {
    pub jellyfin_user_id: String,
    pub username: String,
    pub password: String,
}

pub async fn ensure_library(
    runtime: &Arc<JellyfinRuntime>,
    party_id: i64,
    collection_type: &str,
    root: &str,
) -> Result<String> {
    let name = naming::library_name(party_id);

    if let Some(existing) = runtime.jellyfin.virtual_folder_id(&name).await? {
        return Ok(existing);
    }

    runtime.jellyfin.create_virtual_folder(&name, collection_type, root).await?;

    let id = runtime.jellyfin.virtual_folder_id(&name).await?.ok_or_else(|| {
        crate::error::JellyfinError::Internal(format!(
            "library `{name}` did not appear after creation"
        ))
    })?;

    Ok(id)
}

pub async fn create_guest(
    runtime: &Arc<JellyfinRuntime>,
    party_id: i64,
    discord_id: u64,
    library_item_id: &str,
) -> Result<ProvisionedGuest> {
    let username = naming::guest_username(party_id, discord_id);
    let password = generate_password();

    let created = runtime.jellyfin.create_user(&username, &password).await?;

    runtime
        .jellyfin
        .set_policy(&created.user.id, &UserPolicy::guest(library_item_id.to_owned()))
        .await?;

    Ok(ProvisionedGuest { jellyfin_user_id: created.user.id, username, password })
}
