use zayden_app::entitlement;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tier {
    Free,
    Pro,
    Ultra,
}

impl Tier {
    pub const PAID_LADDER: [Self; 1] = [Self::Pro];

    #[must_use]
    pub fn next_paid(self) -> Option<Self> {
        Self::PAID_LADDER.into_iter().find(|plan| *plan > self)
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Free => "Free",
            Self::Pro => "Pro",
            Self::Ultra => "Ultra",
        }
    }

    #[must_use]
    pub const fn price(self) -> &'static str {
        match self {
            Self::Free => "$0",
            Self::Pro => "$2.99",
            Self::Ultra => "$9.99",
        }
    }

    #[must_use]
    pub const fn upload_limit_mb(self) -> u32 {
        match self {
            Self::Free => 10,
            Self::Pro => 50,
            Self::Ultra => 100,
        }
    }

    #[must_use]
    pub const fn upload_cooldown(self) -> &'static str {
        match self {
            Self::Free => "60 min",
            Self::Pro => "30 min",
            Self::Ultra => "10 min",
        }
    }

    #[must_use]
    pub const fn css_suffix(self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Pro => "pro",
            Self::Ultra => "ultra",
        }
    }

    #[must_use]
    pub const fn as_entitlement(self) -> entitlement::Tier {
        match self {
            Self::Free => entitlement::Tier::Free,
            Self::Pro => entitlement::Tier::Pro,
            Self::Ultra => entitlement::Tier::Ultra,
        }
    }

    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "free" => Some(Self::Free),
            "pro" => Some(Self::Pro),
            "ultra" => Some(Self::Ultra),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserTierInfo {
    pub tier: Option<Tier>,
    pub upgrade_url: Option<String>,
}
