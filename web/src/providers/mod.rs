//! Webhooks and OAuth connect flows for the Ko-fi, Patreon and YouTube
//! integrations.
//!
//! [`routes`] registers every route, and [`origin_policy`] must be the
//! router's origin policy: the webhook posts come from the providers' servers
//! and carry no same-origin proof, so they are exempt from the cross-site
//! check.
//!
//! Webhook posts always answer `200` with an empty body, so a provider never
//! retries a business failure; a malformed Ko-fi form is refused with `415` or
//! `422`. The connect and callback routes sit behind
//! [`require_auth`](crate::auth::require_auth) and answer only with `303`
//! redirects.
//!
//! The routes need the router's cookie layer and the
//! [`WebState`](crate::state::WebState) app context.

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

/// Webhook paths that provider servers post to without an `Origin` header
/// proving a same-site caller.
pub const WEBHOOK_PATHS: [&str; 3] =
    ["/webhooks/kofi", "/webhooks/patreon", "/webhooks/youtube"];

/// The default cross-site check with the webhook paths exempted.
#[must_use]
pub fn origin_policy() -> OriginPolicy {
    OriginPolicy::new().exempt_paths(WEBHOOK_PATHS)
}

/// Registers the provider webhooks, the Patreon and YouTube connect flows and
/// the session guard in front of them.
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
