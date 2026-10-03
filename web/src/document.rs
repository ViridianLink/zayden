use topcoat::Result;
use topcoat::asset::{Asset, asset};
use topcoat::context::Cx;
use topcoat::router::error::NotFoundError;
use topcoat::router::{Slot, StatusCode, layout, try_endpoint};
use topcoat::view::{Child, View, component, error_boundary, view};

use crate::pages::not_found::not_found_page;
use crate::public::{LANDING_TITLE, LOGIN_TITLE, PRIVACY_TITLE, TERMS_TITLE};
use crate::shell::{GUILDS_TITLE, OVERVIEW_TITLE, UPGRADE_TITLE};

pub const STYLESHEET: Asset = asset!(concat!(env!("OUT_DIR"), "/tailwind.css"));

pub const PENDING_SUBMIT: Asset = asset!("../assets/pending-submit.js");

pub const NOT_FOUND_TITLE: &str = "Not found - Zayden";

pub const PAGE_TITLES: &[(&str, &str)] = &[
    ("/{*rest}", NOT_FOUND_TITLE),
    ("/", LANDING_TITLE),
    ("/login", LOGIN_TITLE),
    ("/privacy", PRIVACY_TITLE),
    ("/terms", TERMS_TITLE),
    ("/guilds", GUILDS_TITLE),
    ("/guild/{guild_id}", OVERVIEW_TITLE),
    ("/upgrade", UPGRADE_TITLE),
];

#[must_use]
pub fn page_title(cx: &Cx) -> &'static str {
    try_endpoint(cx)
        .and_then(|endpoint| title_for(endpoint.path().as_str()))
        .unwrap_or(NOT_FOUND_TITLE)
}

fn title_for(pattern: &str) -> Option<&'static str> {
    let pattern = if pattern.is_empty() { "/" } else { pattern };
    PAGE_TITLES.iter().find(|(route, _)| *route == pattern).map(|(_, title)| *title)
}

#[layout("/")]
pub(crate) async fn root_layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
    let title = page_title(cx);

    Ok(view! {
        error_boundary(
            fallback: |error| {
                if error.downcast_ref::<NotFoundError>().is_none() {
                    return Err(error);
                }
                Ok(
                    view! {
                        (StatusCode::NOT_FOUND)
                        document(title: NOT_FOUND_TITLE, not_found_page())
                    },
                )
            },
            document(title: title, (slot))
        )
    })
}

#[component]
async fn document(title: &str, #[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en" data-bot="zayden">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <link rel="stylesheet" href=(STYLESHEET)>
                topcoat::runtime::script()
                <script type="module" src=(PENDING_SUBMIT)></script>
                <title>(title)</title>
            </head>
            <body>(child)</body>
        </html>
    })
}
