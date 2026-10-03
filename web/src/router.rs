use topcoat::cookie::RouterBuilderCookieExt;
use topcoat::router::{Router, RouterBuilder};
use topcoat::runtime::RouterBuilderRuntimeExt;

use crate::{auth, document, pages, providers, public, shell};

const ROUTE_GROUPS: [fn(RouterBuilder) -> RouterBuilder; 4] =
    [public::routes, auth::routes, providers::routes, shell::routes];

#[must_use]
pub fn router(base: RouterBuilder) -> Router {
    let base = base.layout(document::root_layout).page(pages::not_found::not_found);

    ROUTE_GROUPS
        .into_iter()
        .fold(base, |builder, group| group(builder))
        .origin_policy(providers::origin_policy())
        .runtime()
        .cookies()
        .build()
}
