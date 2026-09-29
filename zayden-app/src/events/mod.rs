mod channel;
pub mod listener;

pub use channel::Channel;
use serde::{Deserialize, Serialize};

use crate::entitlement::EntitlementScope;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppEvent {
    ConfigChanged(u64),
    ModulesChanged(u64),
    EntitlementChanged(EntitlementScope),
    PatreonPost(String),
    YoutubeUpload(String),
    HostingPaid(i64),
    ServingChanged(u64),
    CustomBotsChanged(u64),
    LoadoutsChanged,
    Resync,
}
