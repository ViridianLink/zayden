use serde::{Deserialize, Serialize};

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
