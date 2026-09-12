use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;

use super::{
    ENABLED,
    PLUGIN_ID,
    ROUTE,
    accounts_mut,
    is_enabled,
    is_user,
    new_entry,
    normalise_id,
    own_account,
    start_task,
    text_of,
};
use crate::error::{JellyfinError, Result};
use crate::transport::JellyfinClient;
use crate::transport::http::fetch_json;

const TASK_KEY: &str = "SerializdSync";

const ACCOUNTS: &str = "SerializdAccounts";
const EMAIL: &str = "Email";
const PASSWORD: &str = "Password";
const USERNAME: &str = "SerializdUsername";

const DEFAULTS: [(&str, bool); 10] = [
    ("SyncFavorites", true),
    ("EnableDateFilter", false),
    ("IsPrimary", true),
    ("SyncWatchlist", true),
    ("AutoRequestWatchlist", true),
    ("BackfillAvailableRequests", false),
    ("MirrorJellyseerrWatchlist", false),
    ("SkipPreviouslySynced", true),
    ("StopOnFailure", false),
    ("EnableDiaryImport", true),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub username: Option<String>,
    pub enabled: bool,
}

pub async fn account(
    client: &JellyfinClient,
    jellyfin_user_id: &str,
) -> Result<Option<Account>> {
    let config = client.plugin_configuration(PLUGIN_ID).await?;
    Ok(find(&config, jellyfin_user_id))
}

#[must_use]
pub fn find(config: &Value, jellyfin_user_id: &str) -> Option<Account> {
    own_account(config, ACCOUNTS, jellyfin_user_id, EMAIL).map(|entry| Account {
        username: text_of(entry, USERNAME).map(str::to_owned),
        enabled: is_enabled(entry),
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct VerifyRequest<'a> {
    email: &'a str,
    password: &'a str,
}

#[derive(Debug, Deserialize)]
struct VerifyResponse {
    #[serde(default, alias = "Username")]
    username: Option<String>,
}

pub async fn verify(
    client: &JellyfinClient,
    email: &str,
    password: &str,
) -> Result<Option<String>> {
    let body = VerifyRequest { email: email.trim(), password };

    let response = fetch_json::<VerifyResponse, _>(
        crate::transport::jellyfin::SERVICE,
        "Serializd login",
        || client.post(&format!("{ROUTE}/Serializd/Verify")).json(&body),
    )
    .await;

    match response {
        Ok(verified) => Ok(verified
            .username
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty())),
        Err(e) if e.status_code() == Some(StatusCode::BAD_REQUEST) => {
            warn!(detail = %e, "Jellyscribe rejected a Serializd login");
            Err(JellyfinError::SerializdAuth)
        },
        Err(e) => Err(e.into()),
    }
}

pub async fn configure(
    client: &JellyfinClient,
    jellyfin_user_id: &str,
    email: &str,
    password: &str,
    username: Option<&str>,
) -> Result<bool> {
    let mut config = client.plugin_configuration(PLUGIN_ID).await?;

    if !config.is_object() {
        return Err(JellyfinError::Internal(
            "Jellyscribe returned a configuration that is not an object".to_owned(),
        ));
    }

    let created =
        apply_credentials(&mut config, jellyfin_user_id, email, password, username);

    client.set_plugin_configuration(PLUGIN_ID, &config).await?;

    Ok(created)
}

pub fn apply_credentials(
    config: &mut Value,
    jellyfin_user_id: &str,
    email: &str,
    password: &str,
    username: Option<&str>,
) -> bool {
    let wanted = normalise_id(jellyfin_user_id);
    let email = email.trim();

    let Some(accounts) = accounts_mut(config, ACCOUNTS) else {
        return false;
    };

    let slot = accounts.iter().position(|entry| {
        is_user(entry, &wanted)
            && text_of(entry, EMAIL)
                .is_none_or(|known| known.eq_ignore_ascii_case(email))
    });

    let created = slot.is_none();
    let index = slot.unwrap_or_else(|| {
        accounts.push(new_entry(&wanted, &[(EMAIL, email)], true, &DEFAULTS));
        accounts.len() - 1
    });

    if let Some(entry) = accounts.get_mut(index).and_then(Value::as_object_mut) {
        entry.insert(EMAIL.to_owned(), Value::String(email.to_owned()));
        entry.insert(PASSWORD.to_owned(), Value::String(password.to_owned()));
        entry.insert(ENABLED.to_owned(), Value::Bool(true));

        if let Some(name) = username {
            entry.insert(USERNAME.to_owned(), Value::String(name.to_owned()));
        }
    }

    created
}

pub async fn start_sync(client: &JellyfinClient) -> Result<bool> {
    start_task(client, TASK_KEY).await
}
