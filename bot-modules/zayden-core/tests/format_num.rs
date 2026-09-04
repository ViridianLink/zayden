//! Thousands-grouping invariants for `FormatNum`. The implementation formats
//! through a stack `NumBuffer` and groups in a single forward pass, so the
//! cases that matter are the group boundaries (where a separator is or is not
//! emitted) and the signed bounds, where `unsigned_abs` is the only thing
//! keeping `i64::MIN` from overflowing.

use zayden_core::FormatNum;

#[test]
fn single_group_is_left_ungrouped() {
    assert_eq!(0_i64.format(), "0");
    assert_eq!(1_i64.format(), "1");
    assert_eq!(99_i64.format(), "99");
    assert_eq!(999_i64.format(), "999");
}

#[test]
fn separators_land_on_every_third_digit() {
    assert_eq!(1000_i64.format(), "1,000");
    assert_eq!(9999_i64.format(), "9,999");
    assert_eq!(999_999_i64.format(), "999,999");
    assert_eq!(1_000_000_i64.format(), "1,000,000");
    assert_eq!(1_234_567_i64.format(), "1,234,567");
    assert_eq!(12_345_678_i64.format(), "12,345,678");
    assert_eq!(123_456_789_i64.format(), "123,456,789");
}

#[test]
fn the_sign_sits_outside_the_grouping() {
    assert_eq!((-1_i64).format(), "-1");
    assert_eq!((-999_i64).format(), "-999");
    assert_eq!((-1000_i64).format(), "-1,000");
    assert_eq!((-1_234_567_i64).format(), "-1,234,567");
}

#[test]
fn both_widths_survive_their_bounds() {
    assert_eq!(i64::MAX.format(), "9,223,372,036,854,775,807");
    assert_eq!(i64::MIN.format(), "-9,223,372,036,854,775,808");
    assert_eq!(i32::MAX.format(), "2,147,483,647");
    assert_eq!(i32::MIN.format(), "-2,147,483,648");
}
