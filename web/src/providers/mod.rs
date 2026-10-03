mod access;
mod error;
mod fields;
mod kofi;
mod oauth;
mod patreon;
mod reply;
mod youtube;

use topcoat::router::{OriginPolicy, RouterBuilder};

use crate::auth::require_auth;

pub const WEBHOOK_PATHS: [&str; 3] =
    ["/webhooks/kofi", "/webhooks/patreon", "/webhooks/youtube"];

#[must_use]
pub fn origin_policy() -> OriginPolicy {
    OriginPolicy::new().exempt_paths(WEBHOOK_PATHS)
}

#[must_use]
pub fn routes(base: RouterBuilder) -> RouterBuilder {
    base.route(kofi::kofi_webhook_handler)
        .route(patreon::patreon_webhook_handler)
        .route(patreon::patreon_connect_handler)
        .route(patreon::patreon_callback_handler)
        .route(youtube::youtube_verify_handler)
        .route(youtube::youtube_notify_handler)
        .route(youtube::youtube_connect_handler)
        .route(youtube::youtube_callback_handler)
        .layer(require_auth("/patreon"))
        .layer(require_auth("/youtube"))
}
