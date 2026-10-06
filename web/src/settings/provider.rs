use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::request::uri;
use topcoat::view::{View, component, view};

use super::action;
use crate::components::settings::alert;
use crate::util::server_error_text;

pub(super) fn query_value(cx: &Cx, name: &str) -> Option<String> {
    let query = uri(cx).query()?;

    url::form_urlencoded::parse(query.as_bytes())
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

const DISCONNECTED_FLAG: &str = "disconnected";

fn kept_query(cx: &Cx) -> Option<String> {
    let query = uri(cx).query()?;
    let kept: Vec<_> = url::form_urlencoded::parse(query.as_bytes())
        .filter(|(key, _)| key != DISCONNECTED_FLAG)
        .collect();

    (!kept.is_empty()).then(|| {
        url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs(kept)
            .finish()
    })
}

pub(super) fn provider_action(cx: &Cx, guild_id: &str, slug: &str) -> String {
    let action = action(guild_id, slug);

    match kept_query(cx) {
        Some(query) => format!("{action}?{query}"),
        None => action,
    }
}

pub(super) fn disconnected_location(cx: &Cx, guild_id: &str, slug: &str) -> String {
    let action = provider_action(cx, guild_id, slug);
    let separator = if action.contains('?') { '&' } else { '?' };

    format!("{action}{separator}{DISCONNECTED_FLAG}=1")
}

pub(super) fn disconnected(cx: &Cx) -> bool {
    query_value(cx, DISCONNECTED_FLAG).as_deref() == Some("1")
}

pub(super) struct Notice {
    pub(super) class: String,
    pub(super) role: &'static str,
    pub(super) message: String,
}

impl Notice {
    pub(super) fn outcome(class: &str, role: &'static str, message: &str) -> Self {
        Self { class: format!("alert {class}"), role, message: message.to_owned() }
    }

    pub(super) fn disconnect(
        outcome: std::result::Result<(), &str>,
        disconnected: &str,
    ) -> Self {
        match outcome {
            Ok(()) => Self {
                class: "alert success".to_owned(),
                role: "status",
                message: disconnected.to_owned(),
            },
            Err(message) => Self {
                class: "alert error".to_owned(),
                role: "alert",
                message: format!(
                    "Failed to disconnect: {}",
                    server_error_text(message)
                ),
            },
        }
    }
}

#[component]
pub(super) async fn notices(lines: &[Notice]) -> Result<impl View> {
    Ok(view! {
        #[key(index)]
        for (index, notice) in lines.iter().enumerate() {
            alert(
                class: notice.class.as_str(),
                role: notice.role,
                message: notice.message.as_str()
            )
        }
    })
}
