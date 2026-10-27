use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::request::uri;
use topcoat::view::{View, attributes, component, view};

use super::state::plain;
use crate::components::alert::{AlertVariant, alert, alert_title};

pub(super) fn query_value(cx: &Cx, name: &str) -> Option<String> {
    let query = uri(cx).query()?;

    url::form_urlencoded::parse(query.as_bytes())
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

pub(super) struct Banner {
    pub(super) class: &'static str,
    pub(super) role: &'static str,
    pub(super) message: &'static str,
}

#[component]
pub(super) async fn banner(line: Option<&Banner>) -> Result<impl View> {
    Ok(view! {
        if let Some(line) = line {
            alert(
                variant: if line.class == "error" {
                    AlertVariant::Destructive
                } else {
                    AlertVariant::Neutral
                },
                attrs: attributes! { class=(line.class) role=(line.role) },
                alert_title((line.message))
            )
        }
    })
}

#[component]
pub(super) async fn status_unknown(
    provider: &str,
    reason: &str,
    advice: &str,
) -> Result<impl View> {
    Ok(view! {
        <section class="settings-section" aria-labelledby="connection-title">
            <h2 class="label" id="connection-title">"Connection"</h2>
            alert(
                variant: AlertVariant::Neutral,
                attrs: attributes! { class="warning" role="alert" },
                alert_title(
                    (format!(
                        "Couldn't load the {provider} connection: {}",
                        plain(reason),
                    ))
                )
            )
            <p class="page-lead">(advice)</p>
        </section>
    })
}
