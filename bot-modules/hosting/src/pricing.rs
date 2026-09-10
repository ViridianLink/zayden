use zayden_app::entitlement::Tier;

use crate::error::{HostingError, Result};

pub const RUNG_CENTS: i64 = 250;
pub const LADDER_CENTS: [i64; 4] = [250, 500, 750, 1000];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Plan {
    Small,
    Medium,
    Large,
    Xl,
}

impl Plan {
    pub const ALL: [Self; 4] = [Self::Small, Self::Medium, Self::Large, Self::Xl];

    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
            Self::Xl => "xl",
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Small => "Small",
            Self::Medium => "Medium",
            Self::Large => "Large",
            Self::Xl => "XL",
        }
    }

    pub fn from_key(key: &str) -> Result<Self> {
        match key {
            "small" => Ok(Self::Small),
            "medium" => Ok(Self::Medium),
            "large" => Ok(Self::Large),
            "xl" => Ok(Self::Xl),
            other => Err(HostingError::UnknownPlan(other.to_owned())),
        }
    }

    #[must_use]
    pub const fn memory_mib(self) -> i32 {
        match self {
            Self::Small => 2048,
            Self::Medium => 4096,
            Self::Large => 6144,
            Self::Xl => 8192,
        }
    }

    #[must_use]
    pub const fn cpu_percent(self) -> i32 {
        match self {
            Self::Small => 100,
            Self::Medium => 150,
            Self::Large => 200,
            Self::Xl => 250,
        }
    }

    #[must_use]
    pub const fn disk_mib(self) -> i32 {
        match self {
            Self::Small => 10_240,
            Self::Medium => 20_480,
            Self::Large => 30_720,
            Self::Xl => 40_960,
        }
    }

    #[must_use]
    pub const fn rung(self) -> i64 {
        match self {
            Self::Small => 1,
            Self::Medium => 2,
            Self::Large => 3,
            Self::Xl => 4,
        }
    }
}

#[must_use]
pub const fn tier_discount_rungs(tier: Tier) -> i64 {
    match tier {
        Tier::Free => 0,
        Tier::Pro => 1,
        Tier::Ultra => 2,
    }
}

#[must_use]
pub const fn price_cents(plan: Plan, tier: Tier) -> i64 {
    let rungs = plan.rung() - tier_discount_rungs(tier);
    if rungs <= 0 { 0 } else { rungs * RUNG_CENTS }
}

#[must_use]
pub const fn max_servers(tier: Tier) -> i64 {
    match tier {
        Tier::Free => 1,
        Tier::Pro => 2,
        Tier::Ultra => 3,
    }
}

#[must_use]
pub const fn is_included(plan: Plan, tier: Tier) -> bool {
    price_cents(plan, tier) == 0
}

#[must_use]
pub fn plan_for_price(cents: i64, tier: Tier) -> Option<Plan> {
    if cents <= 0 {
        return None;
    }
    Plan::ALL.into_iter().find(|&plan| price_cents(plan, tier) == cents)
}

#[must_use]
pub fn price_from_tier_name(tier_name: &str) -> Option<i64> {
    let digits: String =
        tier_name.chars().filter(|c| c.is_ascii_digit() || *c == '.').collect();
    let cents = parse_amount_cents(&digits)?;
    LADDER_CENTS.contains(&cents).then_some(cents)
}

#[must_use]
pub fn parse_amount_cents(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    let (whole, frac) = raw.split_once('.').map_or((raw, ""), |(w, f)| (w, f));

    let whole: i64 = if whole.is_empty() { 0 } else { whole.parse().ok()? };

    let frac = match frac.len() {
        0 => 0,
        1 => frac.parse::<i64>().ok()? * 10,
        _ => frac.get(..2)?.parse::<i64>().ok()?,
    };

    whole.checked_mul(100)?.checked_add(frac)
}

#[must_use]
pub fn format_price(cents: i64) -> String {
    if cents == 0 {
        return "included".to_owned();
    }
    format!("£{}.{:02}", cents / 100, cents % 100)
}
