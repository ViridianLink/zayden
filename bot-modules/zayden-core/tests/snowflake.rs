//! The one snowflake round-trip the row accessors cannot reach.
//!
//! Every `*Row` accessor that maps a signed `BIGINT` column back to a snowflake
//! goes through `as_u64`, so the ordinary range is already covered where it
//! matters -- see `reaction-roles` and `suggestions` `manager.rs`, which also
//! pin that each accessor reads its own column. Those fixtures use realistic
//! ids, all below `i64::MAX`, where a clamping implementation would still pass.
//! Only the high-bit case distinguishes a bit-preserving reinterpretation from
//! a clamp, and Discord snowflakes do exceed `i64::MAX`.

use zayden_core::{as_i64, as_u64};

#[test]
fn round_trip_high_bit_set() {
    let id: u64 = u64::MAX;
    assert_eq!(as_u64(as_i64(id)), id);
}
