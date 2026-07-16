use serde::{Deserialize, Serialize};
use twilight_model::channel::ChannelType;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Free,
    Pro,
    Enterprise,
}

impl Tier {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Free => "Free",
            Self::Pro => "Pro",
            Self::Enterprise => "Enterprise",
        }
    }

    pub(crate) const fn css_suffix(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Pro => "pro",
            Self::Enterprise => "enterprise",
        }
    }

    pub(crate) const fn is_pro(self) -> bool {
        matches!(self, Self::Pro | Self::Enterprise)
    }

    #[cfg(feature = "ssr")]
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        match key {
            "free" => Some(Self::Free),
            "pro" => Some(Self::Pro),
            "enterprise" => Some(Self::Enterprise),
            _ => None,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserTierInfo {
    pub(crate) tier: Option<Tier>,
    pub(crate) upgrade_url: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct GuildInfo {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) icon: Option<String>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct GuildSettings {
    pub(crate) support_channel_id: Option<String>,
    pub(crate) support_role_id: Option<String>,
    pub(crate) faq_channel_id: Option<String>,
    pub(crate) suggestions_channel_id: Option<String>,
    pub(crate) review_channel_id: Option<String>,
    pub(crate) rules_channel_id: Option<String>,
    pub(crate) general_channel_id: Option<String>,
    pub(crate) spoiler_channel_id: Option<String>,
    pub(crate) artist_role_id: Option<String>,
    pub(crate) sleep_role_id: Option<String>,
    pub(crate) temp_voice_category: Option<String>,
    pub(crate) temp_voice_creator_channel: Option<String>,
    pub(crate) lfg_channel_id: Option<String>,
    pub(crate) lfg_role_id: Option<String>,
    pub(crate) lfg_scheduled_thread_id: Option<String>,
}

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

#[derive(Clone, Serialize, Deserialize)]
pub struct ModuleView {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) description: String,
    pub(crate) commands: Vec<String>,
    pub(crate) enabled: bool,
}
