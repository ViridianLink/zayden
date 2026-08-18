use std::time::Duration;

use zayden_app::entitlement::Tier;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeQuota {
    pub node_budget: usize,
    pub generation_span: i32,
    pub fetch_limit: i64,
    pub max_canvas_pixels: u32,
    pub max_canvas_dim: u32,
    pub avatars: usize,
    pub cooldown: Option<Duration>,
}

impl TreeQuota {
    pub const FREE: Self = Self {
        node_budget: 60,
        generation_span: 2,
        fetch_limit: 500,
        max_canvas_pixels: 2_400_000,
        max_canvas_dim: 2_400,
        avatars: 1,
        cooldown: Some(Duration::from_secs(60)),
    };
    pub const PRO: Self = Self {
        node_budget: 140,
        generation_span: 3,
        fetch_limit: 1_200,
        max_canvas_pixels: 5_000_000,
        max_canvas_dim: 3_200,
        avatars: 12,
        cooldown: Some(Duration::from_secs(20)),
    };
    pub const ULTRA: Self = Self {
        node_budget: 300,
        generation_span: 4,
        fetch_limit: 2_500,
        max_canvas_pixels: 9_000_000,
        max_canvas_dim: 4_096,
        avatars: 60,
        cooldown: None,
    };

    #[must_use]
    pub const fn for_tier(tier: Tier) -> Self {
        match tier {
            Tier::Free => Self::FREE,
            Tier::Pro => Self::PRO,
            Tier::Ultra => Self::ULTRA,
        }
    }

    #[must_use]
    pub const fn raster_limits(&self) -> zayden_graphics::RasterLimits {
        zayden_graphics::RasterLimits {
            max_pixels: self.max_canvas_pixels,
            max_dim: self.max_canvas_dim,
        }
    }

    #[must_use]
    pub const fn next_tier(tier: Tier) -> Option<(Tier, Self)> {
        match tier {
            Tier::Free => Some((Tier::Pro, Self::PRO)),
            Tier::Pro => Some((Tier::Ultra, Self::ULTRA)),
            Tier::Ultra => None,
        }
    }
}
