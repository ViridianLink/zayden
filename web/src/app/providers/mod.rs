use topcoat::router::segment;
mod access;
mod error;
mod fields;
mod kofi;
pub(crate) mod oauth;
mod patreon;
mod reply;
mod youtube;

use topcoat::router::OriginPolicy;

segment!(kind = Group);

pub const WEBHOOK_PATHS: [&str; 3] =
    ["/webhooks/kofi", "/webhooks/patreon", "/webhooks/youtube"];

#[must_use]
pub fn origin_policy() -> OriginPolicy {
    OriginPolicy::new().exempt_paths(WEBHOOK_PATHS)
}
