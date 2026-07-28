use serde::{Deserialize, Serialize};
use twilight_model::channel::ChannelType;

#[derive(Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) kind: ChannelType,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RoleInfo {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) color: u32,
}
