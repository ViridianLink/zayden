use std::sync::Arc;

use sqlx::PgPool;
use topcoat::context::{Cx, try_app_context, try_request_context};
use topcoat::cookie::{CookieJar, CookieJarCell, cookies};
use zayden_app::state::AppState as ZaydenAppState;

use super::error::AuthError;
use crate::state::WebState;

pub fn web_state(cx: &Cx) -> Result<&WebState, AuthError> {
    try_app_context::<WebState>(cx)
        .ok_or(AuthError::MissingContext("missing app state"))
}

pub(super) fn try_web_state(cx: &Cx) -> Option<&WebState> {
    try_app_context::<WebState>(cx)
}

pub fn app_state(cx: &Cx) -> Result<&Arc<ZaydenAppState>, AuthError> {
    web_state(cx).map(|state| &state.app)
}

pub fn db_pool(cx: &Cx) -> Result<&PgPool, AuthError> {
    try_app_context::<WebState>(cx)
        .map(|state| &state.app.db)
        .ok_or(AuthError::MissingContext("missing database pool"))
}

pub fn discord_client(cx: &Cx) -> Result<&Arc<twilight_http::Client>, AuthError> {
    try_app_context::<WebState>(cx)
        .map(|state| &state.discord.http)
        .ok_or(AuthError::MissingContext("missing Discord client"))
}

pub(crate) fn cookie_jar(cx: &Cx) -> Result<&CookieJar, AuthError> {
    if try_request_context::<CookieJarCell>(cx).is_none() {
        return Err(AuthError::MissingContext("missing cookie jar"));
    }
    Ok(cookies(cx))
}
