use crate::dto::destiny2::LoadoutForm;

#[must_use]
pub fn draft_key(id: Option<i32>) -> String {
    id.map_or_else(
        || "zayden:loadout-draft:new".to_owned(),
        |id| format!("zayden:loadout-draft:{id}"),
    )
}

#[cfg(feature = "hydrate")]
fn storage() -> Option<web_sys::Storage> {
    leptos::prelude::window().session_storage().ok().flatten()
}

#[cfg(feature = "hydrate")]
#[must_use]
pub fn load(key: &str) -> Option<LoadoutForm> {
    let json = storage()?.get_item(key).ok()??;
    serde_json::from_str(&json).ok()
}

#[cfg(feature = "hydrate")]
pub fn store(key: &str, form: &LoadoutForm) {
    if let (Some(storage), Ok(json)) = (storage(), serde_json::to_string(form)) {
        let _ = storage.set_item(key, &json);
    }
}

#[cfg(feature = "hydrate")]
pub fn clear(key: &str) {
    if let Some(storage) = storage() {
        let _ = storage.remove_item(key);
    }
}

#[cfg(not(feature = "hydrate"))]
#[must_use]
pub const fn load(_key: &str) -> Option<LoadoutForm> {
    None
}

#[cfg(not(feature = "hydrate"))]
pub const fn store(_key: &str, _form: &LoadoutForm) {}

#[cfg(not(feature = "hydrate"))]
pub const fn clear(_key: &str) {}
