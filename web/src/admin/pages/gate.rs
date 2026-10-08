#![expect(
    unreachable_pub,
    reason = "#[component] re-emits each fn inside its `Component::render` impl, where `pub` cannot be reached; the marker struct the macro emits carries the real, public visibility"
)]

use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::StatusCode;
use topcoat::view::{View, ViewExt, component, view};

use crate::auth::{AuthError, WebRole, require_role};
use crate::components::error_panel::{ErrorAction, error_panel};
use crate::shell::app_shell;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gate {
    Open,
    Forbidden,
    Unavailable(String),
}

pub async fn gate(cx: &Cx, role: WebRole) -> Result<Gate> {
    Ok(match require_role(cx, role).await {
        Ok(_) => Gate::Open,
        Err(error) => {
            let error = error.redirect_unauthenticated()?;
            if error == AuthError::Forbidden {
                Gate::Forbidden
            } else {
                Gate::Unavailable(error.to_string())
            }
        },
    })
}

const fn role_text(role: WebRole) -> &'static str {
    match role {
        WebRole::Admin => "This page is for Zayden's loadout admins.",
        WebRole::Operator => "This page is for Zayden's operators.",
    }
}

#[component]
pub async fn closed(role: WebRole, gate: Gate) -> Result<impl View> {
    let back = [ErrorAction::new("Back to servers", "/guilds")];
    let (status, title, message) = match gate {
        Gate::Unavailable(reason) => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Couldn't check your access",
            format!("Something went wrong: {reason}"),
        ),
        Gate::Open | Gate::Forbidden => (
            StatusCode::FORBIDDEN,
            "You don't have access to this page",
            role_text(role).to_owned(),
        ),
    };

    Ok(view! {
        (status)
        app_shell(
            <div class="page">
                error_panel(title: title, message: &message, actions: &back)
            </div>
        )
    }
    .boxed())
}
