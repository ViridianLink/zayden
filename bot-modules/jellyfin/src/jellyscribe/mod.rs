pub mod letterboxd;
pub mod serializd;

use serde_json::{Map, Value};

use crate::error::Result;
use crate::transport::JellyfinClient;

pub const PLUGIN_ID: &str = "c7a3e1b9-5d42-4f8a-9c06-2b7d8e4f1a35";

const ROUTE: &str = "Jellyfin.Plugin.LetterboxdSync";
const USER_ID: &str = "UserJellyfinId";
const ENABLED: &str = "Enabled";

#[must_use]
pub fn settings_url(base_url: &str) -> String {
    format!("{base_url}/web/configurationpage?name=letterboxduser")
}

async fn start_task(client: &JellyfinClient, key: &str) -> Result<bool> {
    let task = client
        .scheduled_tasks()
        .await?
        .into_iter()
        .find(|task| task.key.as_deref() == Some(key));

    let Some(task) = task else {
        return Ok(false);
    };

    client.start_scheduled_task(&task.id).await?;

    Ok(true)
}

fn own_account<'a>(
    config: &'a Value,
    list: &str,
    jellyfin_user_id: &str,
    identity: &str,
) -> Option<&'a Value> {
    let wanted = normalise_id(jellyfin_user_id);
    let accounts = config.get(list).and_then(Value::as_array)?;

    let mine = || accounts.iter().filter(|entry| is_user(entry, &wanted));
    mine().find(|entry| text_of(entry, identity).is_some()).or_else(|| mine().next())
}

fn accounts_mut<'a>(
    config: &'a mut Value,
    list: &str,
) -> Option<&'a mut Vec<Value>> {
    let slot = config
        .as_object_mut()?
        .entry(list)
        .or_insert_with(|| Value::Array(Vec::new()));

    if !slot.is_array() {
        *slot = Value::Array(Vec::new());
    }

    slot.as_array_mut()
}

fn new_entry(
    jellyfin_user_id: &str,
    fields: &[(&str, &str)],
    enabled: bool,
    defaults: &[(&str, bool)],
) -> Value {
    let mut entry = Map::new();
    entry.insert(USER_ID.to_owned(), Value::String(jellyfin_user_id.to_owned()));

    for (key, value) in fields {
        entry.insert((*key).to_owned(), Value::String((*value).to_owned()));
    }

    entry.insert(ENABLED.to_owned(), Value::Bool(enabled));

    for (flag, value) in defaults {
        entry.insert((*flag).to_owned(), Value::Bool(*value));
    }

    Value::Object(entry)
}

fn is_enabled(entry: &Value) -> bool {
    entry.get(ENABLED).and_then(Value::as_bool).unwrap_or_default()
}

fn is_user(entry: &Value, wanted: &str) -> bool {
    entry
        .get(USER_ID)
        .and_then(Value::as_str)
        .is_some_and(|id| normalise_id(id) == wanted)
}

fn text_of<'a>(entry: &'a Value, key: &str) -> Option<&'a str> {
    entry
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
}

fn normalise_id(raw: &str) -> String {
    raw.trim().replace('-', "").to_ascii_lowercase()
}
