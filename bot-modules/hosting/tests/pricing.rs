//! The ladder is the whole reason four fixed-amount Ko-fi tiers can cover
//! twelve plan/tier combinations. If any reachable price falls off a rung,
//! that price cannot be charged at all.

use hosting::pricing::{
    LADDER_CENTS,
    Plan,
    format_price,
    is_included,
    max_servers,
    parse_amount_cents,
    plan_for_price,
    price_cents,
    price_from_tier_name,
};
use zayden_app::entitlement::Tier;

const TIERS: [Tier; 3] = [Tier::Free, Tier::Pro, Tier::Ultra];

#[test]
fn every_reachable_price_is_a_rung_or_free() {
    for plan in Plan::ALL {
        for tier in TIERS {
            let price = price_cents(plan, tier);
            assert!(
                price == 0 || LADDER_CENTS.contains(&price),
                "{plan:?} at {tier:?} priced {price}, which is off the ladder"
            );
        }
    }
}

#[test]
fn free_tier_prices_match_the_published_table() {
    assert_eq!(price_cents(Plan::Small, Tier::Free), 250);
    assert_eq!(price_cents(Plan::Medium, Tier::Free), 500);
    assert_eq!(price_cents(Plan::Large, Tier::Free), 750);
    assert_eq!(price_cents(Plan::Xl, Tier::Free), 1000);
}

#[test]
fn pro_drops_one_rung_and_ultra_drops_two() {
    assert_eq!(price_cents(Plan::Small, Tier::Pro), 0);
    assert_eq!(price_cents(Plan::Medium, Tier::Pro), 250);
    assert_eq!(price_cents(Plan::Large, Tier::Pro), 500);
    assert_eq!(price_cents(Plan::Xl, Tier::Pro), 750);

    assert_eq!(price_cents(Plan::Small, Tier::Ultra), 0);
    assert_eq!(price_cents(Plan::Medium, Tier::Ultra), 0);
    assert_eq!(price_cents(Plan::Large, Tier::Ultra), 250);
    assert_eq!(price_cents(Plan::Xl, Tier::Ultra), 500);
}

#[test]
fn pro_gets_small_included_and_ultra_gets_medium() {
    assert!(is_included(Plan::Small, Tier::Pro));
    assert!(!is_included(Plan::Medium, Tier::Pro));

    assert!(is_included(Plan::Small, Tier::Ultra));
    assert!(is_included(Plan::Medium, Tier::Ultra));
    assert!(!is_included(Plan::Large, Tier::Ultra));

    assert!(!is_included(Plan::Small, Tier::Free));
}

#[test]
fn a_discount_never_drives_a_price_negative() {
    for plan in Plan::ALL {
        for tier in TIERS {
            assert!(price_cents(plan, tier) >= 0);
        }
    }
}

#[test]
fn a_paid_price_identifies_exactly_one_plan_per_tier() {
    for tier in TIERS {
        for plan in Plan::ALL {
            let price = price_cents(plan, tier);
            if price == 0 {
                // Included plans share a price of zero; they are settled
                // against the subscription, never against a Ko-fi payment.
                assert_eq!(plan_for_price(price, tier), None);
            } else {
                assert_eq!(plan_for_price(price, tier), Some(plan));
            }
        }
    }
}

#[test]
fn tier_names_map_onto_the_ladder() {
    assert_eq!(price_from_tier_name("Hosting £2.50"), Some(250));
    assert_eq!(price_from_tier_name("Hosting 5.00"), Some(500));
    assert_eq!(price_from_tier_name("£7.50 hosting"), Some(750));
    assert_eq!(price_from_tier_name("Hosting £10.00"), Some(1000));
}

#[test]
fn membership_tiers_are_not_mistaken_for_hosting_tiers() {
    // Pro and Ultra must fall through to the entitlement grant, not the
    // hosting branch, or renting a server would buy a subscription.
    assert_eq!(price_from_tier_name("Pro"), None);
    assert_eq!(price_from_tier_name("Ultra"), None);
    assert_eq!(price_from_tier_name("Supporter £3.00"), None);
    assert_eq!(price_from_tier_name(""), None);
}

#[test]
fn amounts_parse_to_pence() {
    assert_eq!(parse_amount_cents("7.50"), Some(750));
    assert_eq!(parse_amount_cents("10"), Some(1000));
    assert_eq!(parse_amount_cents("2.5"), Some(250));
    assert_eq!(parse_amount_cents("0.99"), Some(99));
    assert_eq!(parse_amount_cents("3.999"), Some(399));
    assert_eq!(parse_amount_cents("not money"), None);
}

#[test]
fn caps_rise_with_tier() {
    assert_eq!(max_servers(Tier::Free), 1);
    assert_eq!(max_servers(Tier::Pro), 2);
    assert_eq!(max_servers(Tier::Ultra), 3);
}

#[test]
fn zero_reads_as_included_rather_than_free() {
    assert_eq!(format_price(0), "included");
    assert_eq!(format_price(250), "£2.50");
    assert_eq!(format_price(1000), "£10.00");
}

#[test]
fn plan_keys_round_trip() {
    for plan in Plan::ALL {
        assert_eq!(Plan::from_key(plan.key()).ok(), Some(plan));
    }
    assert!(Plan::from_key("enormous").is_err());
}

#[test]
fn specs_rise_monotonically_with_the_rung() {
    let mut previous = (0, 0, 0);
    for plan in Plan::ALL {
        let current = (plan.memory_mib(), plan.cpu_percent(), plan.disk_mib());
        assert!(current.0 > previous.0);
        assert!(current.1 > previous.1);
        assert!(current.2 > previous.2);
        previous = current;
    }
}
