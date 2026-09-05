pub mod listener;

use serde::{Deserialize, Serialize};

use crate::entitlement::EntitlementScope;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    ConfigChanged(u64),
    EntitlementChanged(EntitlementScope),
    PatreonPost(String),
}
