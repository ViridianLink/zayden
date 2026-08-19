use leptos::prelude::*;
use serde::{Deserialize, Serialize};
#[cfg(feature = "ssr")]
use {
    crate::server::auth::{WebRole, palworld_client, require_role, server_err},
    palworld::save::edit::read_roster,
};

use crate::dto::SaveRoster;

#[derive(Clone, Serialize, Deserialize)]
pub struct SaveRosterView {
    pub roster: SaveRoster,
    pub trait_labels: Vec<(String, String)>,
}

#[server]
pub async fn get_save_roster() -> Result<SaveRosterView, ServerFnError> {
    let _user = require_role(WebRole::Admin).await?;
    let client = palworld_client()?;

    client.refresh_shared_save().await;

    let Some(save_dir) = client.save_dir().map(std::path::Path::to_path_buf) else {
        return Err(ServerFnError::ServerError(
            "no world save is configured for this deployment".to_string(),
        ));
    };
    let level_path = save_dir.join("Level.sav");

    let roster = tokio::task::spawn_blocking(move || {
        let modified = std::fs::metadata(&level_path)?
            .modified()
            .ok()
            .and_then(|t| jiff::Timestamp::try_from(t).ok())
            .map_or(0, jiff::Timestamp::as_second);
        let bytes = std::fs::read(&level_path)?;
        read_roster(&bytes, modified)
    })
    .await
    .map_err(|e| server_err(format!("save read task failed: {e}")))?
    .map_err(server_err)?;

    let roster = SaveRoster::from(roster);
    let trait_labels = labels(&roster).await;

    Ok(SaveRosterView { roster, trait_labels })
}

#[cfg(feature = "ssr")]
async fn labels(roster: &SaveRoster) -> Vec<(String, String)> {
    let Ok(client) = palworld_client() else { return Vec::new() };
    let Ok(passives) = client.passives().await else { return Vec::new() };

    roster
        .trait_ids
        .iter()
        .filter_map(|id| {
            let hit = passives.iter().find(|p| p.key.eq_ignore_ascii_case(id))?;
            Some((id.clone(), hit.name.clone()))
        })
        .collect()
}
