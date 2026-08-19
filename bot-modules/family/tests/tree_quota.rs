//! Tier scaling for `/tree`.
//!
//! These pin the premium contract: the feature works on every tier and each
//! step up only *widens* the expensive axes. They also guard an invariant that
//! would otherwise fail as a hang rather than an error -- see
//! `budget_never_exceeds_what_the_render_semaphore_can_grant`.

use std::time::Duration;

use family::TreeQuota;
use zayden_app::entitlement::Tier;
use zayden_graphics::RENDER_BUDGET_MP;

const TIERS: [(Tier, TreeQuota); 3] = [
    (Tier::Free, TreeQuota::FREE),
    (Tier::Pro, TreeQuota::PRO),
    (Tier::Ultra, TreeQuota::ULTRA),
];

#[test]
fn for_tier_maps_each_tier_to_its_own_quota() {
    for (tier, expected) in TIERS {
        assert_eq!(
            TreeQuota::for_tier(tier),
            expected,
            "{} should map to its own quota",
            tier.as_str(),
        );
    }
}

#[test]
fn every_expensive_axis_widens_with_tier() {
    let (free, pro, ultra) = (TreeQuota::FREE, TreeQuota::PRO, TreeQuota::ULTRA);

    assert!(free.node_budget < pro.node_budget);
    assert!(pro.node_budget < ultra.node_budget);

    assert!(free.generation_span < pro.generation_span);
    assert!(pro.generation_span < ultra.generation_span);

    assert!(free.fetch_limit < pro.fetch_limit);
    assert!(pro.fetch_limit < ultra.fetch_limit);

    assert!(free.max_canvas_pixels < pro.max_canvas_pixels);
    assert!(pro.max_canvas_pixels < ultra.max_canvas_pixels);

    assert!(free.max_canvas_dim < pro.max_canvas_dim);
    assert!(pro.max_canvas_dim < ultra.max_canvas_dim);
}

#[test]
fn cooldown_shortens_with_tier_and_ultra_has_none() {
    let free = TreeQuota::FREE.cooldown.expect("free is rate limited");
    let pro = TreeQuota::PRO.cooldown.expect("pro is rate limited");

    assert!(pro < free, "pro should wait less than free");
    assert_eq!(TreeQuota::ULTRA.cooldown, None, "ultra is not rate limited");
    assert!(free > Duration::ZERO, "a zero cooldown is not a cooldown");
}

/// The free tier has to be a working feature, not a teaser.
///
/// Comparable bots top out around 20 people in one connected family, and the
/// median is far smaller. The free budget is sized so an ordinary family
/// renders whole while the largest ones are where premium starts to pay --
/// which is only meaningful if the budget sits *below* that observed ceiling
/// but comfortably above a typical household.
#[test]
fn the_free_tier_covers_an_ordinary_family() {
    let free = TreeQuota::FREE;

    assert!(
        free.node_budget >= 12,
        "a couple, their parents and a few children must fit on free",
    );
    assert!(
        free.node_budget < 20,
        "a budget at or above the largest observed family would never bind, \
         leaving premium with nothing to offer on this axis",
    );
    assert!(free.generation_span >= 2, "grandparents should be reachable");
}

/// Premium has to actually engage at real family sizes. A budget nobody
/// reaches is a dead knob, which is what the first cut of these numbers was.
#[test]
fn premium_engages_at_realistic_family_sizes() {
    let largest_observed = 20;

    assert!(
        TreeQuota::FREE.node_budget < largest_observed,
        "free must collapse the largest real families, or premium is decorative",
    );
    assert!(
        TreeQuota::PRO.node_budget > largest_observed,
        "pro must render the largest real families whole",
    );
}

/// `fetch_limit` bounds the SQL blast radius, `node_budget` bounds the render.
/// If the fetch were the tighter of the two, collapsing would be driven by the
/// query rather than by the tier, and the "+N more" counts would be wrong.
#[test]
fn the_fetch_limit_never_binds_before_the_node_budget() {
    for (tier, quota) in TIERS {
        let budget = i64::try_from(quota.node_budget).expect("budget fits i64");
        assert!(
            quota.fetch_limit >= budget,
            "{}: fetch limit {} must not bind before the node budget {budget}",
            tier.as_str(),
            quota.fetch_limit,
        );
    }
}

/// Render permits are weighted by megapixels, so a canvas larger than the
/// entire budget would ask `Semaphore::acquire_many` for more permits than it
/// will ever hold. That blocks forever rather than erroring, so the ceiling has
/// to stay inside the budget by construction.
#[test]
fn budget_never_exceeds_what_the_render_semaphore_can_grant() {
    for (tier, quota) in TIERS {
        let weight = quota.max_canvas_pixels.div_ceil(1_000_000);
        assert!(
            weight <= RENDER_BUDGET_MP,
            "{}: a {}px canvas needs {weight} permits but the budget is {RENDER_BUDGET_MP}",
            tier.as_str(),
            quota.max_canvas_pixels,
        );
    }
}

/// A canvas ceiling that a square canvas could never reach would be a dead
/// knob: `max_dim` has to be loose enough for `max_canvas_pixels` to bind.
#[test]
fn the_dimension_ceiling_leaves_room_for_the_pixel_ceiling() {
    for (tier, quota) in TIERS {
        let widest =
            u64::from(quota.max_canvas_dim) * u64::from(quota.max_canvas_dim);
        assert!(
            widest >= u64::from(quota.max_canvas_pixels),
            "{}: max_dim {} cannot reach max_canvas_pixels {}",
            tier.as_str(),
            quota.max_canvas_dim,
            quota.max_canvas_pixels,
        );
    }
}
