//! Round-trip invariants for the `UserId`/`GuildId` <-> `i64` snowflake casts
//! that every DB binding relies on (Discord snowflakes exceed `i64::MAX`, so
//! the cast must be a bit-preserving reinterpretation, not a value clamp).

use zayden_core::{as_i64, as_u64};

#[test]
fn round_trip_zero() {
    assert_eq!(as_u64(as_i64(0)), 0u64);
}

#[test]
fn round_trip_typical_snowflake() {
    let id: u64 = 244_499_782_699_483_136;
    assert_eq!(as_u64(as_i64(id)), id);
}

#[test]
fn round_trip_high_bit_set() {
    let id: u64 = u64::MAX;
    assert_eq!(as_u64(as_i64(id)), id);
}
