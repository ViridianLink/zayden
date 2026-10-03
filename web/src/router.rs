use topcoat::cookie::RouterBuilderCookieExt;
use topcoat::router::{Router, RouterBuilder};
use topcoat::runtime::RouterBuilderRuntimeExt;

use crate::{auth, document, pages, providers, public};

#[must_use]
pub fn router(base: RouterBuilder) -> Router {
    providers::routes(auth::routes(public::routes(
        base.layout(document::root_layout).page(pages::not_found::not_found),
    )))
    .origin_policy(providers::origin_policy())
    .runtime()
    .cookies()
    .build()
}
