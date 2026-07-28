use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct ReactionRoleInfo {
    pub(crate) channel_id: String,
    pub(crate) message_id: String,
    pub(crate) role_id: String,
    pub(crate) emoji: String,
}
