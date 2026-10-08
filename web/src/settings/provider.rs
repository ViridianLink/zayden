use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::request::uri;
use topcoat::view::{View, component, view};

use super::state::plain;

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
            <p class=(line.class) role=(line.role)>(line.message)</p>
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
            <p class="warning" role="alert">
                (format!("Couldn't load the {provider} connection: {}", plain(reason)))
            </p>
            <p class="page-lead">(advice)</p>
        </section>
    })
}
