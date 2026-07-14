//! Unit coverage for the Pelican transport's `modified_at` parsing — the field
//! `refresh_shared_if_stale` compares against the local save's mtime. No live
//! panel is involved; this pins the timestamp-format contract.

use palworld::transport::parse_modified_at;

#[test]
fn parses_iso8601_utc_to_unix_seconds() {
    // 2026-07-14T12:00:00Z == 1_784_030_400 seconds since the epoch.
    assert_eq!(parse_modified_at("2026-07-14T12:00:00Z").unwrap(), 1_784_030_400);
}

#[test]
fn parses_offset_timestamp() {
    // A +01:00 offset is one hour earlier in UTC than the same wall-clock Z.
    let z = parse_modified_at("2026-07-14T12:00:00Z").unwrap();
    let offset = parse_modified_at("2026-07-14T13:00:00+01:00").unwrap();
    assert_eq!(z, offset);
}

#[test]
fn rejects_garbage() {
    assert!(parse_modified_at("not a timestamp").is_err());
}
