use topcoat::cookie::RouterBuilderCookieExt;
use topcoat::router::{Router, RouterBuilder};
use topcoat::runtime::RouterBuilderRuntimeExt;

use crate::{auth, document, pages};

#[must_use]
pub fn router(base: RouterBuilder) -> Router {
    auth::routes(
        base.layout(document::root_layout).page(pages::not_found::not_found),
    )
    .runtime()
    .cookies()
    .build()
}
