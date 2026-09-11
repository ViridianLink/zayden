use reqwest::StatusCode;
use serde::Serialize;
use serde_json::Value;
use tracing::warn;

use crate::error::{JellyfinError, Result};
use crate::transport::JellyfinClient;
use crate::transport::http::send_ok;

pub const PLUGIN_ID: &str = "c7a3e1b9-5d42-4f8a-9c06-2b7d8e4f1a35";

const ROUTE: &str = "Jellyfin.Plugin.LetterboxdSync";
const TASK_KEY: &str = "LetterboxdSync";

const ACCOUNTS: &str = "Accounts";
const USER_ID: &str = "UserJellyfinId";
const USERNAME: &str = "LetterboxdUsername";
const PASSWORD: &str = "LetterboxdPassword";
const ENABLED: &str = "Enabled";

#[must_use]
pub fn settings_url(base_url: &str) -> String {
    format!("{base_url}/web/configurationpage?name=letterboxduser")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub letterboxd_username: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Linked {
    Created,
    Unchanged,
    Conflict(String),
}

pub async fn account(
    client: &JellyfinClient,
    jellyfin_user_id: &str,
) -> Result<Option<Account>> {
    let config = client.plugin_configuration(PLUGIN_ID).await?;
    let wanted = normalise_id(jellyfin_user_id);

    let Some(accounts) = config.get(ACCOUNTS).and_then(Value::as_array) else {
        return Ok(None);
    };

    let mine = || accounts.iter().filter(|entry| is_user(entry, &wanted));
    let entry =
        mine().find(|entry| username_of(entry).is_some()).or_else(|| mine().next());

    Ok(entry.map(view))
}

pub async fn link(
    client: &JellyfinClient,
    jellyfin_user_id: &str,
    letterboxd_username: &str,
) -> Result<Linked> {
    let mut config = client.plugin_configuration(PLUGIN_ID).await?;
    let outcome = apply(&mut config, jellyfin_user_id, letterboxd_username);

    if outcome == Linked::Created {
        client.set_plugin_configuration(PLUGIN_ID, &config).await?;
    }

    Ok(outcome)
}

pub fn apply(
    config: &mut Value,
    jellyfin_user_id: &str,
    letterboxd_username: &str,
) -> Linked {
    let wanted = normalise_id(jellyfin_user_id);
    let target = letterboxd_username.trim();

    let Some(accounts) = accounts_mut(config) else {
        return Linked::Conflict(String::new());
    };

    let mut blank: Option<usize> = None;
    let mut taken: Option<String> = None;

    for (index, entry) in accounts.iter().enumerate() {
        if !is_user(entry, &wanted) {
            continue;
        }

        match username_of(entry) {
            Some(name) if name.eq_ignore_ascii_case(target) => {
                return Linked::Unchanged;
            },
            Some(name) => {
                taken.get_or_insert_with(|| name.to_owned());
            },
            None => {
                blank.get_or_insert(index);
            },
        }
    }

    if let Some(name) = taken {
        return Linked::Conflict(name);
    }

    match blank.and_then(|i| accounts.get_mut(i)).and_then(Value::as_object_mut) {
        Some(entry) => {
            entry.insert(USERNAME.to_owned(), Value::String(target.to_owned()));
        },
        None => accounts.push(new_entry(&wanted, target)),
    }

    Linked::Created
}

fn new_entry(jellyfin_user_id: &str, letterboxd_username: &str) -> Value {
    let mut entry = serde_json::Map::new();
    entry.insert(USER_ID.to_owned(), Value::String(jellyfin_user_id.to_owned()));
    entry.insert(USERNAME.to_owned(), Value::String(letterboxd_username.to_owned()));
    entry.insert(PASSWORD.to_owned(), Value::String(String::new()));
    entry.insert(ENABLED.to_owned(), Value::Bool(false));
    Value::Object(entry)
}

fn accounts_mut(config: &mut Value) -> Option<&mut Vec<Value>> {
    let slot = config
        .as_object_mut()?
        .entry(ACCOUNTS)
        .or_insert_with(|| Value::Array(Vec::new()));

    if !slot.is_array() {
        *slot = Value::Array(Vec::new());
    }

    slot.as_array_mut()
}

fn view(entry: &Value) -> Account {
    Account {
        letterboxd_username: username_of(entry).map(str::to_owned),
        enabled: entry.get(ENABLED).and_then(Value::as_bool).unwrap_or_default(),
    }
}

fn is_user(entry: &Value, wanted: &str) -> bool {
    entry
        .get(USER_ID)
        .and_then(Value::as_str)
        .is_some_and(|id| normalise_id(id) == wanted)
}

fn username_of(entry: &Value) -> Option<&str> {
    entry
        .get(USERNAME)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
}

fn normalise_id(raw: &str) -> String {
    raw.trim().replace('-', "").to_ascii_lowercase()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct Credentials<'a> {
    letterboxd_username: &'a str,
    letterboxd_password: &'a str,
}

pub async fn test_connection(
    client: &JellyfinClient,
    letterboxd_username: &str,
    letterboxd_password: &str,
) -> Result<()> {
    let credentials = Credentials { letterboxd_username, letterboxd_password };

    match send_ok(crate::transport::jellyfin::SERVICE, "Letterboxd login", || {
        client.post(&format!("{ROUTE}/TestConnection")).json(&credentials)
    })
    .await
    {
        Ok(()) => Ok(()),
        Err(e) if e.status_code() == Some(StatusCode::BAD_REQUEST) => {
            warn!(detail = %e, "Jellyscribe rejected a Letterboxd login");
            Err(JellyfinError::LetterboxdAuth)
        },
        Err(e) => Err(e.into()),
    }
}

pub async fn configure(
    client: &JellyfinClient,
    jellyfin_user_id: &str,
    letterboxd_username: &str,
    letterboxd_password: &str,
) -> Result<bool> {
    let mut config = client.plugin_configuration(PLUGIN_ID).await?;

    if !config.is_object() {
        return Err(JellyfinError::Internal(
            "Jellyscribe returned a configuration that is not an object".to_owned(),
        ));
    }

    let created = apply_credentials(
        &mut config,
        jellyfin_user_id,
        letterboxd_username,
        letterboxd_password,
    );

    client.set_plugin_configuration(PLUGIN_ID, &config).await?;

    Ok(created)
}

pub fn apply_credentials(
    config: &mut Value,
    jellyfin_user_id: &str,
    letterboxd_username: &str,
    letterboxd_password: &str,
) -> bool {
    let wanted = normalise_id(jellyfin_user_id);
    let username = letterboxd_username.trim();

    let Some(accounts) = accounts_mut(config) else {
        return false;
    };

    let slot = accounts.iter().position(|entry| {
        is_user(entry, &wanted)
            && username_of(entry)
                .is_none_or(|name| name.eq_ignore_ascii_case(username))
    });

    let created = slot.is_none();
    let index = slot.unwrap_or_else(|| {
        accounts.push(new_entry(&wanted, username));
        accounts.len() - 1
    });

    if let Some(entry) = accounts.get_mut(index).and_then(Value::as_object_mut) {
        entry.insert(USERNAME.to_owned(), Value::String(username.to_owned()));
        entry.insert(
            PASSWORD.to_owned(),
            Value::String(letterboxd_password.to_owned()),
        );
        entry.insert(ENABLED.to_owned(), Value::Bool(true));
    }

    created
}

pub async fn start_sync(client: &JellyfinClient) -> Result<bool> {
    let task = client
        .scheduled_tasks()
        .await?
        .into_iter()
        .find(|task| task.key.as_deref() == Some(TASK_KEY));

    let Some(task) = task else {
        return Ok(false);
    };

    client.start_scheduled_task(&task.id).await?;

    Ok(true)
}
