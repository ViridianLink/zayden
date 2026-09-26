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

#[cfg(feature = "ssr")]
#[derive(Debug, thiserror::Error)]
pub enum ForeignIdError {
    #[error("that channel is not in this server")]
    Channel,
    #[error("that role is not in this server")]
    Role,
}
