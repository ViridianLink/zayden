//! The tier table the upgrade page and tier badge render.

use web::guild::dto::Tier;
use zayden_app::entitlement;

#[test]
fn every_tier_has_its_label_price_and_limits() {
    let rows = [
        (Tier::Free, "Free", "$0", 10, "60 min", "free"),
        (Tier::Pro, "Pro", "$2.99", 50, "30 min", "pro"),
        (Tier::Ultra, "Ultra", "$9.99", 100, "10 min", "ultra"),
    ];

    for (tier, label, price, limit, cooldown, suffix) in rows {
        assert_eq!(tier.label(), label);
        assert_eq!(tier.price(), price);
        assert_eq!(tier.upload_limit_mb(), limit);
        assert_eq!(tier.upload_cooldown(), cooldown);
        assert_eq!(tier.css_suffix(), suffix);
    }
}

#[test]
fn only_pro_is_offered_and_only_below_it() {
    assert_eq!(Tier::PAID_LADDER, [Tier::Pro]);
    assert_eq!(Tier::Free.next_paid(), Some(Tier::Pro));
    assert_eq!(Tier::Pro.next_paid(), None);
    assert_eq!(Tier::Ultra.next_paid(), None);
}

#[test]
fn keys_are_the_lowercase_names() {
    assert_eq!(Tier::from_key("free"), Some(Tier::Free));
    assert_eq!(Tier::from_key("pro"), Some(Tier::Pro));
    assert_eq!(Tier::from_key("ultra"), Some(Tier::Ultra));
    assert_eq!(Tier::from_key("Pro"), None);
    assert_eq!(Tier::from_key(""), None);
}

#[test]
fn tiers_map_to_entitlements_and_back() {
    for tier in [Tier::Free, Tier::Pro, Tier::Ultra] {
        assert_eq!(Tier::from_key(tier.as_entitlement().as_str()), Some(tier));
    }
    assert_eq!(Tier::Pro.as_entitlement(), entitlement::Tier::Pro);
}

#[test]
fn tiers_order_from_free_to_ultra() {
    assert!(Tier::Free < Tier::Pro);
    assert!(Tier::Pro < Tier::Ultra);
}
