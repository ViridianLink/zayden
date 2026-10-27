use topcoat::Result;
use topcoat::context::Cx;
use topcoat::runtime::{link_attrs, prefetch_mode};
use topcoat::view::{View, attributes, component, view};

use super::action::path_segment;
use super::state::plain;
use crate::auth::AuthError;
use crate::components::button::{ButtonSize, ButtonVariant, button_variants};
use crate::components::error_panel::ErrorAction;
use crate::engagement::EngagementError;
use crate::guild::GuildError;

pub(super) fn load_problem(
    guild_id: &str,
    retry: &str,
    error: &EngagementError,
) -> (String, Vec<ErrorAction>) {
    let back = ErrorAction::new("Back to servers", "/guilds");
    let auth = if let EngagementError::Auth(auth)
    | EngagementError::Guild(GuildError::Auth(auth)) = error
    {
        Some(auth)
    } else {
        None
    };

    if auth == Some(&AuthError::BotNotInGuild) {
        let invite = format!("/invite?guild={}", path_segment(guild_id));
        ("Zayden isn't in this server yet.".to_owned(), vec![
            ErrorAction::new("Add Zayden to this server", &invite).external(),
            back,
        ])
    } else if auth == Some(&AuthError::InvalidGuildId) {
        ("That address doesn't name a Discord server.".to_owned(), vec![back])
    } else if error.is_denied() {
        (
            "You need Manage Server in this server to open this page.".to_owned(),
            vec![back],
        )
    } else {
        (format!("Something went wrong: {}", plain(&error.to_string())), vec![
            ErrorAction::new("Try again", retry),
            back,
        ])
    }
}

#[component]
pub(super) async fn load_error(
    cx: &Cx,
    title: &str,
    message: &str,
    actions: &[ErrorAction],
) -> Result<impl View> {
    Ok(view! {
        <section class="error-panel" role="alert" aria-labelledby="load-error-title">
            <h2 class="error-title" id="load-error-title">(title)</h2>
            <p class="error-text">(message)</p>
            <div class="error-actions">
                #[key(index)]
                for (index, action) in actions.iter().enumerate() {
                    let variant = if index == 0 {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Secondary
                    };
                    let link = if action.external {
                        attributes! { href=(action.href.as_str()) rel="external" }
                    } else {
                        link_attrs(cx, action.href.as_str(), prefetch_mode(cx))
                    };
                    <a class=(button_variants(variant, ButtonSize::Md)) (link)>
                        (action.label.as_str())
                    </a>
                }
            </div>
        </section>
    })
}
