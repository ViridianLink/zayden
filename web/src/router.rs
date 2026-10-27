use topcoat::cookie::RouterBuilderCookieExt;
use topcoat::router::{Router, RouterBuilder};
use topcoat::runtime::{PrefetchMode, RouterBuilderRuntimeExt};

use crate::{
    account,
    admin,
    auth,
    document,
    engagement,
    pages,
    providers,
    public,
    settings,
    shell,
};

const ROUTE_GROUPS: [fn(RouterBuilder) -> RouterBuilder; 10] = [
    document::routes,
    public::routes,
    auth::routes,
    providers::routes,
    account::routes,
    shell::routes,
    settings::routes,
    engagement::pages::routes,
    admin::pages::routes,
    admin::editor::routes,
];

#[must_use]
pub fn router(base: RouterBuilder) -> Router {
    let base = base.layout(document::root_layout).page(pages::not_found::not_found);

    ROUTE_GROUPS
        .into_iter()
        .fold(base, |builder, group| group(builder))
        .origin_policy(providers::origin_policy())
        .runtime()
        .prefetch(PrefetchMode::Never)
        .cookies()
        .build()
}
