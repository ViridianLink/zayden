//! Milestone 4 coverage for the premium-entitlement plumbing.
//!
//! These exercise the pure, DB-free logic that the Ko-fi webhook and the
//! Discord entitlement handler depend on: token verification, the 32-day
//! subscription grant window, tier ordering (which the runtime gate and the
//! expiry sweep both key on), and the provider scope mapping that makes Ko-fi
//! and Discord converge on the same per-user `Tier::Pro`.
//!
//! The end-to-end sweep/grant round-trip is DB-bound and is covered by the
//! spec's manual "Done when" checks against a live Postgres.

use jiff::{SignedDuration, Timestamp};
use zayden_app::entitlement::{
    DiscordProvider,
    EntitlementScope,
    KoFiPayload,
    KoFiProvider,
    KoFiType,
    Tier,
};

fn subscription_payload(verification_token: &str) -> String {
    format!(
        r#"{{
            "verification_token": "{verification_token}",
            "kofi_transaction_id": "tx-1",
            "email": "Supporter@Example.com",
            "type": "Subscription",
            "is_subscription_payment": true,
            "is_first_subscription_payment": true,
            "timestamp": "2026-07-15T00:00:00Z",
            "message_id": null
        }}"#
    )
}

// ── Tier ordering & round-trip ────────────────────────────────────────────────

#[test]
fn tier_is_ordered_free_pro_ultra() {
    assert!(Tier::Free < Tier::Pro);
    assert!(Tier::Pro < Tier::Ultra);
    // The runtime gate is `effective >= required`: Pro clears a Pro gate,
    // Ultra clears it too, Free does not.
    assert!(Tier::Pro >= Tier::Pro);
    assert!(Tier::Ultra >= Tier::Pro);
    assert!(Tier::Free < Tier::Pro);
}

#[test]
fn tier_round_trips_through_str() {
    for tier in [Tier::Free, Tier::Pro, Tier::Ultra] {
        assert_eq!(tier.as_str().parse(), Ok(tier));
    }
    assert_eq!("free".parse(), Ok(Tier::Free));
    assert_eq!("pro".parse(), Ok(Tier::Pro));
    assert_eq!("ultra".parse(), Ok(Tier::Ultra));
    assert!("platinum".parse::<Tier>().is_err());
}

// ── Ko-fi payload parsing & verification ──────────────────────────────────────

#[test]
fn payload_parses_subscription_with_verification_token() {
    let payload: KoFiPayload =
        serde_json::from_str(&subscription_payload("secret-abc")).unwrap();
    assert_eq!(payload.verification_token, "secret-abc");
    assert_eq!(payload.kind, KoFiType::Subscription);
    assert!(payload.is_subscription_payment);
}

#[test]
fn payload_verification_token_defaults_when_absent() {
    // A forged payload can simply omit the field; it must default to empty so
    // it never matches a configured secret.
    let raw = r#"{
        "kofi_transaction_id": "tx-2",
        "email": "a@b.com",
        "type": "Donation",
        "is_subscription_payment": false,
        "is_first_subscription_payment": false,
        "timestamp": "2026-07-15T00:00:00Z"
    }"#;
    let payload: KoFiPayload = serde_json::from_str(raw).unwrap();
    assert_eq!(payload.verification_token, "");
    assert_eq!(payload.kind, KoFiType::Donation);
}

#[test]
fn kofi_type_shop_order_rename_parses() {
    let raw = r#"{
        "verification_token": "s",
        "kofi_transaction_id": "tx-3",
        "email": "a@b.com",
        "type": "Shop Order",
        "is_subscription_payment": false,
        "is_first_subscription_payment": false,
        "timestamp": "2026-07-15T00:00:00Z"
    }"#;
    let payload: KoFiPayload = serde_json::from_str(raw).unwrap();
    assert_eq!(payload.kind, KoFiType::ShopOrder);
}

#[test]
fn verification_ok_only_on_exact_match_with_configured_secret() {
    let payload: KoFiPayload =
        serde_json::from_str(&subscription_payload("right-secret")).unwrap();

    // Matches the configured secret.
    assert!(payload.verification_ok(Some("right-secret")));
    // Wrong secret is rejected (forgery).
    assert!(!payload.verification_ok(Some("wrong-secret")));
    // No secret configured -> reject everything rather than trust the payload.
    assert!(!payload.verification_ok(None));
}

// ── Ko-fi subscription grant window ───────────────────────────────────────────

#[test]
fn subscription_expiry_is_thirty_two_days_out() {
    let now = Timestamp::now();
    let expires = KoFiProvider::subscription_expiry(now).expect("no overflow");

    let delta = expires.duration_since(now);
    assert_eq!(delta, SignedDuration::from_hours(32 * 24));
    assert_eq!(KoFiProvider::SUBSCRIPTION_GRACE_DAYS, 32);
    // A renewal received later pushes the window strictly forward.
    let later = now.checked_add(SignedDuration::from_hours(24)).unwrap();
    let renewed = KoFiProvider::subscription_expiry(later).unwrap();
    assert!(renewed > expires);
}

// ── Provider scope convergence (both providers -> per-user Pro) ────────────────

#[test]
fn discord_user_entitlement_maps_to_per_user_pro() {
    let grant = DiscordProvider::build_grant(555, Some(42), None, None)
        .expect("user-scoped grant");
    assert_eq!(grant.scope, EntitlementScope::User(42));
    assert_eq!(grant.tier, Tier::Pro);
    assert_eq!(grant.external_id, "555");
    assert!(grant.expires_at.is_none());
}

#[test]
fn discord_grant_maps_remaining_scopes() {
    let guild = DiscordProvider::build_grant(1, None, Some(7), None).unwrap();
    assert_eq!(guild.scope, EntitlementScope::Guild(7));

    let both = DiscordProvider::build_grant(2, Some(3), Some(9), None).unwrap();
    assert_eq!(both.scope, EntitlementScope::UserInGuild(3, 9));

    // Neither user nor guild -> nothing to grant.
    assert!(DiscordProvider::build_grant(3, None, None, None).is_none());
}

#[test]
fn discord_grant_converts_ends_at_to_expiry() {
    let ends_at = 1_800_000_000; // fixed unix second
    let grant = DiscordProvider::build_grant(4, Some(1), None, Some(ends_at))
        .expect("grant");
    assert_eq!(grant.expires_at, Timestamp::from_second(ends_at).ok());
}
