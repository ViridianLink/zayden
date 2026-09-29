#![cfg(feature = "ssr")]
//! New Zayden emojis come from an admin's link or file. The image type is taken
//! from its bytes, the size is capped at Discord's limit before upload, and a
//! link may only reach public https hosts, since the server fetches it.

use std::net::IpAddr;

use dashboard::server::destiny2::reserved_keys;
use dashboard::server::emoji_upload::{
    MAX_EMOJI_BYTES,
    data_uri,
    decode_data_uri,
    https_url,
    is_public,
    sniff,
};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";

#[test]
fn images_are_identified_by_magic_bytes() {
    assert_eq!(sniff(PNG), Some("image/png"));
    assert_eq!(sniff(&[0xff, 0xd8, 0xff, 0xe0]), Some("image/jpeg"));
    assert_eq!(sniff(b"GIF89a...."), Some("image/gif"));
    assert_eq!(sniff(b"RIFF\0\0\0\0WEBPVP8 "), Some("image/webp"));
    assert_eq!(sniff(b"<svg xmlns="), None);
    assert_eq!(sniff(b""), None);
}

#[test]
fn a_data_uri_round_trips_and_is_retyped_from_its_bytes() {
    let uri = data_uri(PNG).unwrap();
    assert!(uri.starts_with("data:image/png;base64,"));
    let lying = uri.replace("image/png", "image/gif");
    assert_eq!(decode_data_uri(&lying).unwrap(), PNG);
}

#[test]
fn oversized_or_unknown_images_are_refused() {
    let mut big = PNG.to_vec();
    big.resize(MAX_EMOJI_BYTES + 1, 0);
    assert!(data_uri(&big).is_err());
    assert!(data_uri(b"plain text").is_err());
}

#[test]
fn malformed_data_uris_are_refused() {
    assert!(decode_data_uri("data:image/png,rawbytes").is_err());
    assert!(decode_data_uri("data:image/png;base64,***").is_err());
    assert!(decode_data_uri("https://example.com/a.png").is_err());
}

#[test]
fn only_https_links_are_fetched() {
    assert!(https_url("https://www.bungie.net/icon.png").is_ok());
    assert!(https_url("http://www.bungie.net/icon.png").is_err());
    assert!(https_url("ftp://example.com/a.png").is_err());
    assert!(https_url("not a url").is_err());
}

#[test]
fn private_and_special_addresses_are_not_public() {
    for ip in [
        "127.0.0.1",
        "10.0.3.119",
        "172.16.0.1",
        "192.168.1.1",
        "169.254.169.254",
        "100.64.0.1",
        "0.0.0.0",
        "::1",
        "::",
        "fc00::1",
        "fe80::1",
        "::ffff:127.0.0.1",
    ] {
        assert!(!is_public(ip.parse::<IpAddr>().unwrap()), "{ip}");
    }
    assert!(is_public("1.1.1.1".parse().unwrap()));
    assert!(is_public("2606:4700::1111".parse().unwrap()));
}

#[test]
fn enum_icon_keys_are_reserved() {
    let reserved = reserved_keys();
    for key in
        ["titan", "solar", "pve", "kinetic", "auto_rifle", "class_item", "grenade"]
    {
        assert!(reserved.iter().any(|k| k == key), "{key}");
    }
    assert!(!reserved.iter().any(|k| k == "spark_of_shock"));
}
