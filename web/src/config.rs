use std::env;
use std::net::{AddrParseError, SocketAddr};

use zayden_app::config::BotConfig;

pub const BIND_ADDR_ENV: &str = "DASHBOARD_BIND_ADDR";

pub fn bind_addr(config: &BotConfig) -> Result<SocketAddr, AddrParseError> {
    resolve_bind_addr(env::var(BIND_ADDR_ENV).ok().as_deref(), &config.bind_addr)
}

pub fn resolve_bind_addr(
    env_override: Option<&str>,
    configured: &str,
) -> Result<SocketAddr, AddrParseError> {
    env_override
        .map(str::trim)
        .filter(|addr| !addr.is_empty())
        .unwrap_or(configured)
        .parse()
}
