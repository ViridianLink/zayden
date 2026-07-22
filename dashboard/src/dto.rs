use serde::{Deserialize, Serialize};
use twilight_model::channel::ChannelType;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Free,
    Pro,
    Ultra,
}

impl Tier {
    pub(crate) const PAID_LADDER: [Self; 1] = [Self::Pro];

    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Free => "Free",
            Self::Pro => "Pro",
            Self::Ultra => "Ultra",
        }
    }

    pub(crate) const fn price(self) -> &'static str {
        match self {
            Self::Free => "$0",
            Self::Pro => "$2.99",
            Self::Ultra => "$9.99",
        }
    }

    pub(crate) const fn upload_limit_mb(self) -> u32 {
        match self {
            Self::Free => 10,
            Self::Pro => 50,
            Self::Ultra => 100,
        }
    }

    pub(crate) const fn upload_cooldown(self) -> &'static str {
        match self {
            Self::Free => "60 min",
            Self::Pro => "30 min",
            Self::Ultra => "10 min",
        }
    }

    pub(crate) const fn css_suffix(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Pro => "pro",
            Self::Ultra => "ultra",
        }
    }

    #[cfg(feature = "ssr")]
    pub(crate) fn from_key(key: &str) -> Option<Self> {
        match key {
            "free" => Some(Self::Free),
            "pro" => Some(Self::Pro),
            "ultra" => Some(Self::Ultra),
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
    pub(crate) family_max_partners: String,
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
