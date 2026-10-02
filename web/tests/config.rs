use std::net::SocketAddr;

use web::config::resolve_bind_addr;

#[test]
fn configured_address_is_used_without_override() {
    let addr = resolve_bind_addr(None, "0.0.0.0:3000").unwrap();
    assert_eq!(addr, "0.0.0.0:3000".parse::<SocketAddr>().unwrap());
}

#[test]
fn env_override_wins() {
    let addr = resolve_bind_addr(Some("127.0.0.1:3100"), "0.0.0.0:3000").unwrap();
    assert_eq!(addr, "127.0.0.1:3100".parse::<SocketAddr>().unwrap());
}

#[test]
fn blank_override_falls_back_to_config() {
    let addr = resolve_bind_addr(Some("  "), "0.0.0.0:3000").unwrap();
    assert_eq!(addr, "0.0.0.0:3000".parse::<SocketAddr>().unwrap());
}

#[test]
fn invalid_override_is_an_error() {
    assert!(resolve_bind_addr(Some("localhost"), "0.0.0.0:3000").is_err());
}
