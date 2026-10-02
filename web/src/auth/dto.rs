use twilight_model::channel::ChannelType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionUser {
    pub id: String,
    pub name: String,
    pub avatar: Option<String>,
}

impl SessionUser {
    #[must_use]
    pub fn avatar_url(&self) -> Option<String> {
        self.avatar.as_ref().map(|hash| {
            format!(
                "https://cdn.discordapp.com/avatars/{}/{}.png?size=64",
                self.id, hash,
            )
        })
    }

    #[must_use]
    pub fn initial(&self) -> String {
        self.name.chars().next().unwrap_or('#').to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelInfo {
    pub id: String,
    pub name: String,
    pub kind: ChannelType,
    pub tags: Vec<ForumTagInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForumTagInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleInfo {
    pub id: String,
    pub name: String,
    pub color: u32,
}
