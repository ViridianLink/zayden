use serde::{Deserialize, Serialize};
use twilight_model::channel::ChannelType;

#[derive(Clone, Serialize, Deserialize)]
pub struct ChannelInfo {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) kind: ChannelType,
    pub(crate) tags: Vec<ForumTagInfo>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ForumTagInfo {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct RoleInfo {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) color: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SessionUser {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) avatar: Option<String>,
}

impl SessionUser {
    pub(crate) fn avatar_url(&self) -> Option<String> {
        self.avatar.as_ref().map(|hash| {
            format!(
                "https://cdn.discordapp.com/avatars/{}/{}.png?size=64",
                self.id, hash,
            )
        })
    }

    pub(crate) fn initial(&self) -> String {
        self.name.chars().next().unwrap_or('#').to_string()
    }
}
