use reqwest::StatusCode;
use serde::Serialize;
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
use crate::transport::http::send_ok;

const TASK_KEY: &str = "LetterboxdSync";

const ACCOUNTS: &str = "Accounts";
const USERNAME: &str = "LetterboxdUsername";
const PASSWORD: &str = "LetterboxdPassword";

const DEFAULTS: [(&str, bool); 10] = [
    ("SyncFavorites", true),
    ("EnableDateFilter", false),
    ("IsPrimary", true),
    ("EnableWatchlistSync", true),
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

    Ok(own_account(&config, ACCOUNTS, jellyfin_user_id, USERNAME).map(|entry| {
        Account {
            username: text_of(entry, USERNAME).map(str::to_owned),
            enabled: is_enabled(entry),
        }
    }))
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

    let Some(accounts) = accounts_mut(config, ACCOUNTS) else {
        return Linked::Conflict(String::new());
    };

    let mut blank: Option<usize> = None;
    let mut taken: Option<String> = None;

    for (index, entry) in accounts.iter().enumerate() {
        if !is_user(entry, &wanted) {
            continue;
        }

        match text_of(entry, USERNAME) {
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
        None => accounts.push(new_account(&wanted, target, false)),
    }

    Linked::Created
}

fn new_account(jellyfin_user_id: &str, username: &str, enabled: bool) -> Value {
    new_entry(
        jellyfin_user_id,
        &[(USERNAME, username), (PASSWORD, "")],
        enabled,
        &DEFAULTS,
    )
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "PascalCase")]
struct TestRequest<'a> {
    letterboxd_username: &'a str,
    letterboxd_password: &'a str,
}

pub async fn test_connection(
    client: &JellyfinClient,
    letterboxd_username: &str,
    letterboxd_password: &str,
) -> Result<()> {
    let body = TestRequest { letterboxd_username, letterboxd_password };

    match send_ok(crate::transport::jellyfin::SERVICE, "Letterboxd login", || {
        client.post(&format!("{ROUTE}/TestConnection")).json(&body)
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

    let Some(accounts) = accounts_mut(config, ACCOUNTS) else {
        return false;
    };

    let slot = accounts.iter().position(|entry| {
        is_user(entry, &wanted)
            && text_of(entry, USERNAME)
                .is_none_or(|name| name.eq_ignore_ascii_case(username))
    });

    let created = slot.is_none();
    let index = slot.unwrap_or_else(|| {
        accounts.push(new_account(&wanted, username, true));
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
    start_task(client, TASK_KEY).await
}
