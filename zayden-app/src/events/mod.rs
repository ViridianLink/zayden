pub mod listener;

use serde::{Deserialize, Serialize};

use crate::entitlement::EntitlementScope;

/// Cross-process broadcast events for cache invalidation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    ConfigChanged(u64),
    EntitlementChanged(EntitlementScope),
}
