use leptos::prelude::ServerFnError;

pub(crate) const UNAUTHENTICATED: &str = "unauthenticated";
pub(crate) const FORBIDDEN: &str = "forbidden";

/// Whether the server refused the caller, as opposed to failing to serve them.
pub(crate) fn is_denied(e: &ServerFnError) -> bool {
    let ServerFnError::ServerError(msg) = e else {
        return false;
    };

    matches!(msg.as_str(), UNAUTHENTICATED | FORBIDDEN)
}
