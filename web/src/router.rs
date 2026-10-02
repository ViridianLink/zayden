use topcoat::cookie::RouterBuilderCookieExt;
use topcoat::router::{Router, RouterBuilder};
use topcoat::runtime::RouterBuilderRuntimeExt;

use crate::{auth, document, pages};

/// Registers every layout, page and route on `base`.
///
/// `base` carries what the handlers read from app context: the asset
/// configuration and, in the server binary, the
/// [`WebState`](crate::state::WebState).
#[must_use]
pub fn router(base: RouterBuilder) -> Router {
    auth::routes(
        base.layout(document::root_layout).page(pages::not_found::not_found),
    )
    .runtime()
    .cookies()
    .build()
}
